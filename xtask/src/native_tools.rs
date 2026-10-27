// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use crate::{
    Result,
    records::{self, Artifact},
};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(deny_unknown_fields)]
struct Llvm {
    layout_version: u32,
    version: String,
    release: Artifact,
    package: Artifact,
    tool_identity: Artifact,
    asset_digest: String,
    selection_profile: String,
    installation: String,
    binding_generator: bool,
    libclang: Option<String>,
    cc: String,
    cxx: String,
    linker: String,
    ldflags: String,
    rustflags: Vec<String>,
    scope: String,
}
// The vendor may add fields; only our own evidence schema is closed.
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    digest: String,
    browser_download_url: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    version: String,
    asset_digest: String,
    selection_profile: String,
    files: Vec<File>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    path: String,
    sha256: String,
}

fn validate(record: &Llvm, release: &Release, identity: &Identity) -> Result<()> {
    records::version(record.layout_version)?;
    let version = semver::Version::parse(&record.version)?;
    if !version.pre.is_empty()
        || !version.build.is_empty()
        || release.draft
        || release.prerelease
        || release.tag_name != format!("llvmorg-{version}")
        || identity.version != record.version
        || identity.asset_digest != record.asset_digest
        || identity.selection_profile != record.selection_profile
        || record.selection_profile != "c-native-bindgen-v1"
        || record.scope.trim().is_empty()
    {
        return Err("LLVM record differs from the selected formal release/cache profile".into());
    }
    let name = record
        .package
        .path
        .rsplit('/')
        .next()
        .ok_or("LLVM package name missing")?;
    if ![
        format!("LLVM-{version}-Linux-X64.tar.zst"),
        format!("LLVM-{version}-Linux-X64.tar.xz"),
    ]
    .contains(&name.to_owned())
    {
        return Err("LLVM package does not match the native Linux x64 release".into());
    }
    let assets: Vec<_> = release.assets.iter().filter(|a| a.name == name).collect();
    if assets.len() != 1
        || assets[0].digest != record.asset_digest
        || record.asset_digest != format!("sha256:{}", record.package.sha256)
        || !assets[0]
            .browser_download_url
            .starts_with("https://github.com/llvm/llvm-project/releases/download/")
    {
        return Err("LLVM package identity differs from its maintainer release asset".into());
    }
    let mut files = BTreeMap::new();
    for file in &identity.files {
        if file.path.is_empty()
            || file.path.starts_with('/')
            || file.path.contains(['\\', ':', '\0'])
            || file
                .path
                .split('/')
                .any(|p| p.is_empty() || p == ".." || p == ".")
            || file.sha256.len() != 64
            || !file.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || files
                .insert(file.path.as_str(), file.sha256.as_str())
                .is_some()
        {
            return Err("LLVM cache has invalid or duplicate file identities".into());
        }
    }
    let prefix = format!("{}/", record.installation);
    if !record.installation.starts_with('/') {
        return Err("LLVM native installation path missing".into());
    }
    for (path, relative) in [
        (&record.cc, "bin/clang"),
        (&record.cxx, "bin/clang++"),
        (&record.linker, "bin/ld.lld"),
    ] {
        if path.strip_prefix(&prefix) != Some(relative) || !files.contains_key(relative) {
            return Err(
                "LLVM selected compiler/linker is outside the recorded package installation".into(),
            );
        }
    }
    match (record.binding_generator, &record.libclang) {
        (true, Some(path)) => {
            let library = path
                .strip_prefix(&prefix)
                .ok_or("libclang is outside its recorded installation")?;
            if !library.starts_with("lib/")
                || !library.rsplit('/').next().is_some_and(|n| {
                    n.starts_with("libclang") && n.contains(".so") && !n.contains("cpp")
                })
                || !files.contains_key(library)
            {
                return Err("LLVM libclang C API identity missing".into());
            }
        }
        (false, None) => {}
        _ => return Err("LLVM generator selection differs from its library evidence".into()),
    }
    let flags = [
        "-C".into(),
        format!("linker={}", record.cc),
        "-C".into(),
        "link-arg=-fuse-ld=lld".into(),
    ];
    if !record.rustflags.ends_with(&flags)
        || record.ldflags.split_whitespace().last() != Some("-fuse-ld=lld")
    {
        return Err("LLVM native link flags do not select the recorded compiler/linker".into());
    }
    Ok(())
}

pub(crate) fn verify(root: &Path, artifact: &Artifact) -> Result<()> {
    let record: Llvm = records::read(&records::verify(root, artifact)?)?;
    let release: Release = records::read(&records::verify(root, &record.release)?)?;
    records::verify(root, &record.package)?;
    let identity: Identity = records::read(&records::verify(root, &record.tool_identity)?)?;
    validate(&record, &release, &identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> (Llvm, Release, Identity) {
        let artifact = |path| json!({"path":path,"sha256":"0".repeat(64)});
        let record = json!({"layout_version":1,"version":"1.2.3","release":artifact("release.json"),"package":artifact("inputs/LLVM-1.2.3-Linux-X64.tar.xz"),"tool_identity":artifact("identity.json"),"asset_digest":format!("sha256:{}","0".repeat(64)),"selection_profile":"c-native-bindgen-v1","installation":"/deps/llvm","binding_generator":true,"libclang":"/deps/llvm/lib/libclang.so","cc":"/deps/llvm/bin/clang","cxx":"/deps/llvm/bin/clang++","linker":"/deps/llvm/bin/ld.lld","ldflags":"-fuse-ld=lld","rustflags":["-C","linker=/deps/llvm/bin/clang","-C","link-arg=-fuse-ld=lld"],"scope":"native C compiler and binding generator"});
        let release = json!({"tag_name":"llvmorg-1.2.3","draft":false,"prerelease":false,"assets":[{"name":"LLVM-1.2.3-Linux-X64.tar.xz","digest":format!("sha256:{}","0".repeat(64)),"browser_download_url":"https://github.com/llvm/llvm-project/releases/download/llvmorg-1.2.3/package"}]});
        let files = ["bin/clang", "bin/clang++", "bin/ld.lld", "lib/libclang.so"]
            .map(|path| json!({"path":path,"sha256":"0".repeat(64)}));
        let identity = json!({"version":"1.2.3","asset_digest":format!("sha256:{}","0".repeat(64)),"selection_profile":"c-native-bindgen-v1","files":files});
        (
            serde_json::from_value(record).unwrap(),
            serde_json::from_value(release).unwrap(),
            serde_json::from_value(identity).unwrap(),
        )
    }
    #[test]
    fn rejects_prereleases_wrong_archives_and_unmatched_installed_tools() {
        let (mut record, mut release, identity) = fixture();
        assert!(validate(&record, &release, &identity).is_ok());
        record.binding_generator = false;
        assert!(validate(&record, &release, &identity).is_err());
        record.libclang = None;
        assert!(validate(&record, &release, &identity).is_ok());
        record.binding_generator = true;
        assert!(validate(&record, &release, &identity).is_err());
        record.libclang = Some("/deps/llvm/lib/libclang.so".into());
        release.prerelease = true;
        assert!(validate(&record, &release, &identity).is_err());
        release.prerelease = false;
        record.cc = "/system/clang".into();
        assert!(validate(&record, &release, &identity).is_err());
        record.cc = "/deps/llvm/bin/clang".into();
        record.package.path = "inputs/LLVM-1.2.3-Linux-ARM64.tar.xz".into();
        assert!(validate(&record, &release, &identity).is_err());
        record.package.path = "inputs/LLVM-1.2.3-Linux-X64.tar.xz".into();
        release.assets[0].digest = "sha256:other".into();
        assert!(validate(&record, &release, &identity).is_err());
    }
    #[test]
    fn rejects_escaped_cache_entries_and_false_acceptance_fields() {
        let (record, release, mut identity) = fixture();
        identity.files[0].path = "../clang".into();
        assert!(validate(&record, &release, &identity).is_err());
        let mut invalid = serde_json::to_value(&record).unwrap();
        invalid["phase_accepted"] = true.into();
        assert!(serde_json::from_value::<Llvm>(invalid).is_err());
    }
}
