// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use crate::{
    Result,
    records::{self, Artifact},
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

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
#[cfg_attr(test, derive(serde::Serialize))]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
struct Asset {
    name: String,
    digest: String,
    browser_download_url: String,
}
#[derive(Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(deny_unknown_fields)]
struct Identity {
    version: String,
    asset_digest: String,
    selection_profile: String,
    files: Vec<File>,
}
#[derive(Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(deny_unknown_fields)]
struct File {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
struct PolicyHeader {
    policy: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InstalledLlvm {
    layout_version: u32,
    policy: String,
    version: String,
    installation: String,
    tool_identity: Artifact,
    latest_advisory: Artifact,
    binding_generator: bool,
    libclang: Option<String>,
    cc: String,
    cxx: String,
    linker: String,
    ldflags: String,
    rustflags: Vec<String>,
    scope: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InstalledIdentity {
    policy: String,
    version: String,
    files: Vec<File>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Advisory {
    exit: i32,
    available: bool,
    #[serde(rename = "output")]
    _output: String,
}

fn native_unix_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.contains(['\\', '\0', '\r', '\n'])
        && path
            .split('/')
            .skip(1)
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn libclang_c_api(name: &str) -> bool {
    let Some((base, suffix)) = name.split_once(".so") else {
        return false;
    };
    let base_ok = base == "libclang"
        || base
            .strip_prefix("libclang-")
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
    base_ok
        && (suffix.is_empty()
            || (suffix.starts_with('.')
                && suffix.split('.').skip(1).all(|digits| {
                    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
                })))
}

fn validate_installed(
    record: &InstalledLlvm,
    identity: &InstalledIdentity,
    advisory: &Advisory,
) -> Result<()> {
    records::version(record.layout_version)?;
    if record.policy != "preinstalled"
        || identity.policy != record.policy
        || identity.version != record.version
        || !record
            .version
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_digit)
        || !record.version.contains('.')
        || !record
            .version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_+".contains(&b))
        || !native_unix_path(&record.installation)
        || record.scope.trim().is_empty()
        || advisory.available != (advisory.exit == 0)
    {
        return Err("Invalid preinstalled LLVM identity or advisory status".into());
    }
    // The lookup body may be unavailable, empty or an error response. It is
    // archived as advisory output and never establishes a latest-version gate.
    let prefix = format!("{}/", record.installation);
    let mut expected: BTreeSet<_> = [
        "clang",
        "clang++",
        "llvm-config",
        "ld.lld",
        "llvm-ar",
        "llvm-ranlib",
    ]
    .map(|name| format!("{prefix}bin/{name}"))
    .into_iter()
    .collect();
    for (path, name) in [
        (&record.cc, "clang"),
        (&record.cxx, "clang++"),
        (&record.linker, "ld.lld"),
    ] {
        if path != &format!("{prefix}bin/{name}") {
            return Err(
                "Selected preinstalled LLVM compiler/linker differs from its installation".into(),
            );
        }
    }
    if let Some(library) = &record.libclang {
        let relative = library
            .strip_prefix(&prefix)
            .ok_or("Preinstalled libclang escaped its installation")?;
        if !native_unix_path(library)
            || !relative.starts_with("lib/")
            || !relative.rsplit('/').next().is_some_and(libclang_c_api)
        {
            return Err("Preinstalled libclang C API path differs".into());
        }
        if record.binding_generator {
            expected.insert(library.clone());
        }
        // Early preinstalled records also reported an available, unused
        // libclang when binding_generator=false. It never proves generation.
    } else if record.binding_generator {
        return Err("Selected preinstalled binding generator has no libclang evidence".into());
    }
    let mut actual = BTreeSet::new();
    for file in &identity.files {
        if !native_unix_path(&file.path)
            || !file.path.starts_with(&prefix)
            || file.sha256.len() != 64
            || !file.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || !actual.insert(file.path.clone())
        {
            return Err("Invalid or duplicate preinstalled LLVM file identity".into());
        }
    }
    if actual != expected {
        return Err("Preinstalled LLVM evidence omits or adds selected tool files".into());
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
        return Err("Preinstalled LLVM link flags do not select its compiler/linker".into());
    }
    Ok(())
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
    let bytes = fs::read(records::verify(root, artifact)?)?;
    let header: PolicyHeader = serde_json::from_slice(&bytes)?;
    if let Some(policy) = header.policy {
        if policy != "preinstalled" {
            return Err("Unknown LLVM record selection policy".into());
        }
        // Deserialize the original bytes so duplicate as well as unknown
        // fields remain errors; a Value round-trip would discard duplicates.
        let record: InstalledLlvm = serde_json::from_slice(&bytes)?;
        let identity = records::read(&records::verify(root, &record.tool_identity)?)?;
        let advisory = records::read(&records::verify(root, &record.latest_advisory)?)?;
        return validate_installed(&record, &identity, &advisory);
    }
    let record: Llvm = serde_json::from_slice(&bytes)?;
    let release: Release = records::read(&records::verify(root, &record.release)?)?;
    records::verify(root, &record.package)?;
    let identity: Identity = records::read(&records::verify(root, &record.tool_identity)?)?;
    validate(&record, &release, &identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn test_root() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "mfs-native-tool-record-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        path
    }
    fn write(root: &Path, name: &str, value: &impl serde::Serialize) -> Artifact {
        let path = root.join(name);
        records::create_json(&path, value).unwrap();
        records::artifact(root, &path).unwrap()
    }
    fn installed_fixture(root: &Path) -> serde_json::Value {
        let files = [
            "bin/clang",
            "bin/clang++",
            "bin/llvm-config",
            "bin/ld.lld",
            "bin/llvm-ar",
            "bin/llvm-ranlib",
            "lib/libclang.so.1",
        ]
        .map(|path| json!({"path":format!("/usr/lib/llvm-20/{path}"),"sha256":"0".repeat(64)}));
        let identity = write(
            root,
            "identity.json",
            &json!({"policy":"preinstalled","version":"20.1.8","files":files}),
        );
        let advisory = write(
            root,
            "advisory.json",
            &json!({"exit":22,"available":false,"output":"release lookup unavailable"}),
        );
        json!({"layout_version":1,"policy":"preinstalled","version":"20.1.8","installation":"/usr/lib/llvm-20","tool_identity":identity,"latest_advisory":advisory,
            "binding_generator":true,"libclang":"/usr/lib/llvm-20/lib/libclang.so.1","cc":"/usr/lib/llvm-20/bin/clang","cxx":"/usr/lib/llvm-20/bin/clang++","linker":"/usr/lib/llvm-20/bin/ld.lld",
            "ldflags":"-fuse-ld=lld","rustflags":["-C","linker=/usr/lib/llvm-20/bin/clang","-C","link-arg=-fuse-ld=lld"],"scope":"Unit record consistency only; no native execution or tool-body verification"})
    }
    #[test]
    fn reads_complete_preinstalled_records_without_requiring_a_latest_release() {
        let root = test_root();
        let mut record = installed_fixture(&root);
        assert!(verify(&root, &write(&root, "toolchain.json", &record)).is_ok());
        // Even a successfully retrieved newer release is only opaque advisory
        // output; it must not turn an older installed tool into a failed gate.
        record["latest_advisory"] = serde_json::to_value(write(
            &root,
            "newer.json",
            &json!({"exit":0,"available":true,"output":"{\"tag_name\":\"llvmorg-23.1.4\"}"}),
        ))
        .unwrap();
        assert!(verify(&root, &write(&root, "older-installed.json", &record)).is_ok());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn preserves_the_unused_library_observation_without_accepting_it_as_a_generator() {
        let root = test_root();
        let mut record = installed_fixture(&root);
        let mut identity: serde_json::Value = records::read(&root.join("identity.json")).unwrap();
        identity["files"].as_array_mut().unwrap().pop();
        record["tool_identity"] =
            serde_json::to_value(write(&root, "without-generator.json", &identity)).unwrap();
        assert!(verify(&root, &write(&root, "missing-active-library.json", &record)).is_err());
        record["binding_generator"] = false.into();
        // The first CI producer reported a discovered but unused library.
        assert!(
            verify(
                &root,
                &write(&root, "historical-unused-library.json", &record)
            )
            .is_ok()
        );
        record["libclang"] = serde_json::Value::Null;
        assert!(verify(&root, &write(&root, "inactive-library.json", &record)).is_ok());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_incomplete_changed_escaped_and_falsely_accepted_preinstalled_records() {
        let root = test_root();
        let record = installed_fixture(&root);
        let mut changed = record.clone();
        changed["phase_accepted"] = true.into();
        assert!(verify(&root, &write(&root, "accepted.json", &changed)).is_err());
        changed = record.clone();
        changed["policy"] = "other".into();
        assert!(verify(&root, &write(&root, "policy.json", &changed)).is_err());
        changed = record.clone();
        changed["installation"] = "/usr/lib/../outside".into();
        assert!(verify(&root, &write(&root, "escape.json", &changed)).is_err());
        changed = record.clone();
        changed["cc"] = "/system/clang".into();
        assert!(verify(&root, &write(&root, "compiler.json", &changed)).is_err());
        changed = record.clone();
        changed["libclang"] = serde_json::Value::Null;
        assert!(verify(&root, &write(&root, "missing-library.json", &changed)).is_err());
        changed = record.clone();
        changed["rustflags"] = json!(["-C", "linker=other"]);
        assert!(verify(&root, &write(&root, "flags.json", &changed)).is_err());
        for (number, name) in ["libclang.fake.so", "libclang-cpp.so.20", "libclang.so.bad"]
            .iter()
            .enumerate()
        {
            changed = record.clone();
            changed["libclang"] = format!("/usr/lib/llvm-20/lib/{name}").into();
            assert!(
                verify(
                    &root,
                    &write(&root, &format!("invalid-api-{number}.json"), &changed)
                )
                .is_err()
            );
        }
        let identity_path = root.join("identity.json");
        let identity: serde_json::Value = records::read(&identity_path).unwrap();
        let mut bad_identity = identity.clone();
        bad_identity["files"].as_array_mut().unwrap().pop();
        changed = record.clone();
        changed["tool_identity"] =
            serde_json::to_value(write(&root, "partial-identity.json", &bad_identity)).unwrap();
        assert!(verify(&root, &write(&root, "partial.json", &changed)).is_err());
        bad_identity = identity.clone();
        bad_identity["files"][0]["sha256"] = "bad".into();
        changed = record.clone();
        changed["tool_identity"] =
            serde_json::to_value(write(&root, "bad-hash-identity.json", &bad_identity)).unwrap();
        assert!(verify(&root, &write(&root, "bad-hash.json", &changed)).is_err());
        bad_identity = identity;
        bad_identity["files"][1] = bad_identity["files"][0].clone();
        changed = record.clone();
        changed["tool_identity"] =
            serde_json::to_value(write(&root, "duplicate-identity.json", &bad_identity)).unwrap();
        assert!(verify(&root, &write(&root, "duplicate-files.json", &changed)).is_err());
        let raw = serde_json::to_string(&record).unwrap().replacen(
            "\"policy\":\"preinstalled\"",
            "\"policy\":\"preinstalled\",\"policy\":\"preinstalled\"",
            1,
        );
        let duplicate = root.join("duplicate-fields.json");
        fs::write(&duplicate, raw).unwrap();
        assert!(verify(&root, &records::artifact(&root, &duplicate).unwrap()).is_err());
        fs::write(identity_path, b"changed").unwrap();
        assert!(verify(&root, &write(&root, "changed-identity.json", &record)).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn reads_the_complete_historical_downloaded_format_through_the_same_entrypoint() {
        let root = test_root();
        let (mut record, mut release, mut identity) = fixture();
        let package = root.join("LLVM-1.2.3-Linux-X64.tar.xz");
        fs::write(
            &package,
            b"unit package identity, not a native LLVM installation",
        )
        .unwrap();
        record.package = records::artifact(&root, &package).unwrap();
        record.asset_digest = format!("sha256:{}", record.package.sha256);
        identity.asset_digest = record.asset_digest.clone();
        release.assets[0].digest = record.asset_digest.clone();
        record.release = write(&root, "release.json", &release);
        record.tool_identity = write(&root, "downloaded-identity.json", &identity);
        assert!(verify(&root, &write(&root, "downloaded.json", &record)).is_ok());
        fs::remove_dir_all(root).unwrap();
    }
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
