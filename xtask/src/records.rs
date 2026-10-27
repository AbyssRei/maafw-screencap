// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use crate::Result;

pub const LAYOUT_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub layout_version: u32,
    pub baselines: Vec<Artifact>,
    pub patches: Vec<Artifact>,
    pub runs: Vec<Artifact>,
}

pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

/// Create an immutable record. Never overwrite a baseline or a previous result.
pub fn create_json(path: &Path, record: &impl Serialize) -> Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let mut bytes = serde_json::to_vec_pretty(record)?;
    bytes.push(b'\n');
    file.write_all(&bytes)?;
    Ok(())
}

pub fn hash(path: &Path) -> Result<String> {
    let mut input = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    use std::fmt::Write;
    let mut encoded = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(&mut encoded, "{byte:02x}")?;
    }
    Ok(encoded)
}

pub fn artifact(root: &Path, path: &Path) -> Result<Artifact> {
    let relative = path
        .strip_prefix(root)?
        .to_str()
        .ok_or("non-UTF-8 record path")?
        .replace('\\', "/");
    resolve(root, &relative)?;
    Ok(Artifact {
        path: relative,
        sha256: hash(path)?,
    })
}

/// Paths use portable slash-separated components, relative to the reference root.
/// Reject lexical escapes and canonical escapes (including links/junctions).
pub fn resolve(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty() || relative.contains(['\\', ':', '\0']) {
        return Err(format!("invalid reference path: {relative:?}").into());
    }
    let path = Path::new(relative);
    if relative
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("reference path escapes root: {relative:?}").into());
    }
    let joined = root.join(path);
    let canonical_root = root.canonicalize()?;
    if !joined.canonicalize()?.starts_with(canonical_root) {
        return Err(format!("reference path resolves outside root: {relative:?}").into());
    }
    Ok(joined)
}

pub fn verify(root: &Path, artifact: &Artifact) -> Result<PathBuf> {
    if artifact.sha256.len() != 64
        || !artifact
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(format!("invalid SHA-256 for {}", artifact.path).into());
    }
    let path = resolve(root, &artifact.path)?;
    if hash(&path)? != artifact.sha256 {
        return Err(format!("SHA-256 mismatch: {}", artifact.path).into());
    }
    Ok(path)
}

pub fn version(value: u32) -> Result<()> {
    if value != LAYOUT_VERSION {
        return Err(format!("unsupported reference layout version: {value}").into());
    }
    Ok(())
}

pub fn external_root(path: &Path) -> Result<PathBuf> {
    let product = crate::repository().canonicalize()?;
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    if let Some(ancestor) = absolute.ancestors().find(|part| part.exists())
        && ancestor.canonicalize()?.starts_with(&product)
    {
        return Err("reference root must be outside the product tree".into());
    }
    fs::create_dir_all(path)?;
    let root = path.canonicalize()?;
    if root.starts_with(&product) || product.starts_with(&root) {
        return Err("reference root must be separate from the product tree".into());
    }
    // Git for Windows/MSYS does not accept Rust's verbatim path spelling as an
    // output filename. Identity/containment was checked in canonical form above.
    #[cfg(windows)]
    if let Some(value) = root.to_str() {
        if let Some(unc) = value.strip_prefix("\\\\?\\UNC\\") {
            return Ok(PathBuf::from(format!("\\\\{unc}")));
        }
        if let Some(disk) = value.strip_prefix("\\\\?\\") {
            return Ok(PathBuf::from(disk));
        }
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "mfs-records-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::write(path.join("input"), b"abc").unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn rejects_path_traversal_and_platform_specific_paths() {
        let fixture = Fixture::new();
        for path in [
            "",
            "../input",
            "/input",
            "a/../../input",
            "C:/input",
            "a\\input",
            "./input",
            "a//input",
            "input/",
        ] {
            assert!(resolve(&fixture.0, path).is_err(), "accepted {path:?}");
        }
    }

    #[test]
    fn detects_modified_inputs() {
        let fixture = Fixture::new();
        let record = artifact(&fixture.0, &fixture.0.join("input")).unwrap();
        assert_eq!(
            record.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        verify(&fixture.0, &record).unwrap();
        fs::write(fixture.0.join("input"), b"tampered").unwrap();
        assert!(verify(&fixture.0, &record).is_err());
    }

    #[test]
    fn never_overwrites_archived_records() {
        let fixture = Fixture::new();
        let path = fixture.0.join("manifest.json");
        create_json(&path, &Index::default()).unwrap();
        assert!(create_json(&path, &Index::default()).is_err());
    }

    #[test]
    fn rejects_unknown_layout_versions_and_fields() {
        assert!(version(0).is_err());
        assert!(version(2).is_err());
        assert!(
            serde_json::from_str::<Artifact>(
                r#"{"path":"input","sha256":"x","unrecognized":true}"#
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_outside_reference_root() {
        let inside = Fixture::new();
        let outside = Fixture::new();
        std::os::unix::fs::symlink(outside.0.join("input"), inside.0.join("escape")).unwrap();
        assert!(resolve(&inside.0, "escape").is_err());
    }
}
