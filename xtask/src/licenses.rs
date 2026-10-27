// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// Evidence inventory, not a license compatibility or distribution approval.
use crate::{
    Result, batch,
    records::{self, Artifact, Index},
    repository,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Resolve,
}
#[derive(Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}
#[derive(Deserialize)]
struct Node {
    id: String,
}
#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    source: Option<String>,
    license: Option<String>,
    license_file: Option<String>,
}
#[derive(Deserialize)]
struct Checksum {
    files: BTreeMap<String, String>,
    package: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    layout_version: u32,
    parent_batch: Artifact,
    target: String,
    #[serde(default)]
    document_policy: DocumentPolicy,
    components: Vec<Component>,
    remaining: Vec<String>,
}

// Earlier records used broad prefix discovery. Preserve their interpretation
// without treating ordinary source modules such as copying.rs as text evidence
// in new inventories. Neither policy establishes license applicability.
#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum DocumentPolicy {
    #[default]
    LegacyPrefix,
    TextDocuments,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Component {
    package_id: String,
    name: String,
    version: String,
    source: String,
    declared_license: Option<String>,
    declared_license_file: Option<String>,
    registry_checksum: String,
    documents: Vec<Document>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Document {
    original_path: String,
    artifact: Artifact,
}

fn selected(metadata: &Metadata) -> Result<Vec<&Package>> {
    let packages: BTreeMap<_, _> = metadata.packages.iter().map(|p| (&p.id, p)).collect();
    let nodes: BTreeSet<_> = metadata.resolve.nodes.iter().map(|n| &n.id).collect();
    if packages.len() != metadata.packages.len() || nodes.len() != metadata.resolve.nodes.len() {
        return Err("duplicate Cargo package or resolution node".into());
    }
    let mut result = Vec::new();
    for id in nodes {
        let package = packages
            .get(id)
            .ok_or("Cargo node has no package metadata")?;
        if let Some(source) = &package.source {
            if !source.starts_with("registry+") {
                return Err(
                    "nonregistry dependencies need a separate source/license inventory".into(),
                );
            }
            result.push(*package);
        }
    }
    Ok(result)
}

fn document_path(path: &str, policy: DocumentPolicy) -> bool {
    if matches!(policy, DocumentPolicy::TextDocuments) {
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if [
            "rs", "c", "h", "cc", "cpp", "hpp", "m", "mm", "py", "js", "ts", "sh", "ps1", "cmake",
        ]
        .contains(&extension.as_str())
        {
            return false;
        }
    }
    path.split('/').any(|part| {
        let name = part.to_ascii_lowercase();
        [
            "license",
            "licence",
            "copying",
            "notice",
            "copyright",
            "unlicense",
        ]
        .iter()
        .any(|prefix| {
            name.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(policy, DocumentPolicy::LegacyPrefix)
                    || suffix.is_empty()
                    || suffix == "s"
                    || suffix.starts_with(['.', '-', '_'])
            })
        })
    })
}

fn parent<'a>(run: &'a batch::Run, suffix: &str) -> Result<&'a Artifact> {
    let matches: Vec<_> = run
        .inputs
        .iter()
        .filter(|a| a.path.ends_with(suffix))
        .collect();
    if matches.len() != 1 {
        return Err(format!("license inventory requires one archived {suffix}").into());
    }
    Ok(matches[0])
}

fn retained(root: &Path, run: &batch::Run, path: &Path) -> Result<std::path::PathBuf> {
    let relative = path.to_string_lossy().replace('\\', "/");
    let artifacts: Vec<_> = run.inputs.iter().filter(|a| a.path == relative).collect();
    if artifacts.len() != 1 {
        return Err("license evidence must be retained exactly once in its parent batch".into());
    }
    records::verify(root, artifacts[0])
}

fn validate(root: &Path, inventory: &Inventory) -> Result<()> {
    records::version(inventory.layout_version)?;
    let run: batch::Run = records::read(&records::verify(root, &inventory.parent_batch)?)?;
    records::verify(root, parent(&run, "/inputs/Cargo.lock")?)?;
    let metadata_artifact = parent(&run, "/inputs/cargo-metadata.json")?;
    let metadata: Metadata = records::read(&records::verify(root, metadata_artifact)?)?;
    let target = fs::read_to_string(records::verify(
        root,
        parent(&run, "/inputs/dependency-target.txt")?,
    )?)?;
    if inventory.target.trim().is_empty()
        || inventory.target != target.trim()
        || inventory.remaining.is_empty()
    {
        return Err("license inventory lacks its target or outstanding review scope".into());
    }
    let packages = selected(&metadata)?;
    let components: BTreeMap<_, _> = inventory
        .components
        .iter()
        .map(|c| (&c.package_id, c))
        .collect();
    if components.len() != inventory.components.len() || components.len() != packages.len() {
        return Err("license inventory duplicates or omits resolved registry packages".into());
    }
    let base = Path::new(&metadata_artifact.path).parent().unwrap();
    for package in packages {
        let component = components
            .get(&package.id)
            .ok_or("license inventory omits a resolved package")?;
        if component.name != package.name
            || component.version != package.version
            || Some(&component.source) != package.source.as_ref()
            || component.declared_license != package.license
            || component.declared_license_file != package.license_file
        {
            return Err("license inventory differs from archived Cargo metadata".into());
        }
        let directory = base
            .join("vendor")
            .join(format!("{}-{}", package.name, package.version));
        let checksum: Checksum = records::read(&retained(
            root,
            &run,
            &directory.join(".cargo-checksum.json"),
        )?)?;
        if checksum.package.as_ref() != Some(&component.registry_checksum) {
            return Err("license inventory registry checksum differs from vendor".into());
        }
        let mut expected: BTreeSet<_> = checksum
            .files
            .keys()
            .filter(|path| document_path(path, inventory.document_policy))
            .cloned()
            .collect();
        if let Some(path) = &package.license_file {
            if !checksum.files.contains_key(path) {
                return Err("declared license-file is absent from archived vendor".into());
            }
            expected.insert(path.clone());
        }
        let actual: BTreeSet<_> = component
            .documents
            .iter()
            .map(|d| d.original_path.clone())
            .collect();
        if actual != expected || actual.len() != component.documents.len() {
            return Err("license inventory omits or duplicates vendor license documents".into());
        }
        for document in &component.documents {
            let source = retained(root, &run, &directory.join(&document.original_path))?;
            let archived = records::verify(root, &document.artifact)?;
            if records::hash(&source)? != checksum.files[&document.original_path]
                || fs::read(source)? != fs::read(archived)?
            {
                return Err("license document differs from its archived crate source".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn verify(root: &Path, artifact: &Artifact) -> Result<()> {
    validate(root, &records::read(&records::verify(root, artifact)?)?)
}

pub fn run(args: &[String]) -> Result<()> {
    let (input, root) =
        match args {
            [input] => (
                input,
                env::var_os("MFS_REFERENCE_ROOT")
                    .map(Into::into)
                    .unwrap_or_else(|| {
                        repository()
                            .parent()
                            .unwrap()
                            .join("maafw-screencap-reference")
                    }),
            ),
            [input, flag, root] if flag == "--root" => (input, root.into()),
            _ => return Err(
                "expected reference licenses <Rust batch manifest> [--root <external directory>]"
                    .into(),
            ),
        };
    let root = records::external_root(&root)?;
    let index: Index = records::read(&root.join("manifest.json"))?;
    records::version(index.layout_version)?;
    let artifact = index
        .runs
        .iter()
        .find(|a| &a.path == input)
        .ok_or("license input must be an indexed Rust batch")?;
    batch::verify(&root, artifact)?;
    let run: batch::Run = records::read(&records::verify(&root, artifact)?)?;
    let metadata_artifact = parent(&run, "/inputs/cargo-metadata.json")?;
    let metadata: Metadata = records::read(&records::verify(&root, metadata_artifact)?)?;
    let target = fs::read_to_string(records::verify(
        &root,
        parent(&run, "/inputs/dependency-target.txt")?,
    )?)?;
    let id = format!(
        "p0-licenses-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let output = root.join("runs").join(&id);
    fs::create_dir(&output)?;
    let inputs = output.join("inputs");
    fs::create_dir(&inputs)?;
    let mut inventory = Inventory {
        layout_version: records::LAYOUT_VERSION,
        parent_batch: artifact.clone(),
        target: target.trim().into(),
        document_policy: DocumentPolicy::TextDocuments,
        components: Vec::new(),
        remaining: vec![
            "Declared SPDX expressions and named license documents are evidence, not a compatibility/distribution approval; review embedded source notices, generated code and required corresponding source/relink materials.".into(),
            "This target-filtered registry graph includes build/test tools and optional capabilities; native/system/toolchain dependencies, fixed upstream/PR sources and final asset contents require separate review.".into(),
        ],
    };
    let base = Path::new(&metadata_artifact.path).parent().unwrap();
    for (number, package) in selected(&metadata)?.into_iter().enumerate() {
        let directory = base
            .join("vendor")
            .join(format!("{}-{}", package.name, package.version));
        let checksum: Checksum = records::read(&records::resolve(
            &root,
            &directory
                .join(".cargo-checksum.json")
                .to_string_lossy()
                .replace('\\', "/"),
        )?)?;
        let mut documents: BTreeSet<_> = checksum
            .files
            .keys()
            .filter(|path| document_path(path, inventory.document_policy))
            .cloned()
            .collect();
        if let Some(path) = &package.license_file {
            documents.insert(path.clone());
        }
        let mut component = Component {
            package_id: package.id.clone(),
            name: package.name.clone(),
            version: package.version.clone(),
            source: package.source.clone().unwrap(),
            declared_license: package.license.clone(),
            declared_license_file: package.license_file.clone(),
            registry_checksum: checksum
                .package
                .ok_or("registry vendor has no package checksum")?,
            documents: Vec::new(),
        };
        if package.license.as_ref().is_none_or(|l| l.trim().is_empty()) {
            inventory.remaining.push(format!("{} {} has no declared license expression; inspect its source/repository licensing.", package.name, package.version));
        }
        if documents.is_empty() {
            inventory.remaining.push(format!("{} {} has no discovered standalone license document; inspect its source/repository licensing.", package.name, package.version));
        }
        for path in documents {
            let source = records::resolve(
                &root,
                &directory.join(&path).to_string_lossy().replace('\\', "/"),
            )?;
            // The source resolution above rejects absolute/escaping components
            // before the same relative crate path is used for the copy.
            let destination = inputs.join(format!("component-{number}")).join(&path);
            fs::create_dir_all(destination.parent().unwrap())?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)?;
            std::io::copy(&mut fs::File::open(source)?, &mut file)?;
            drop(file);
            component.documents.push(Document {
                original_path: path,
                artifact: records::artifact(&root, &destination)?,
            });
        }
        inventory.components.push(component);
    }
    validate(&root, &inventory)?;
    let inventory_path = inputs.join("registry-licenses.json");
    records::create_json(&inventory_path, &inventory)?;
    let log = output.join("license-evidence.log");
    fs::write(
        &log,
        format!(
            "Inventoried {} registry packages for {} from the archived target graph. Exact metadata, document coverage, vendor checksums and copied bytes checked. No license compatibility, final artifact or P0 acceptance asserted.\n",
            inventory.components.len(),
            inventory.target
        ),
    )?;
    let mut result = batch::Run {
        layout_version: records::LAYOUT_VERSION,
        run_id: id,
        phase: "P0".into(),
        phase_status: "incomplete".into(),
        checks: vec![batch::Check {
            name: "registry-license-evidence".into(),
            status: "passed".into(),
            log: records::artifact(&root, &log)?,
        }],
        inputs: vec![artifact.clone(), records::artifact(&root, &inventory_path)?],
        pending: inventory.remaining.clone(),
    };
    batch::snapshot_source(&root, &inputs, &mut result)?;
    result.inputs.extend(
        inventory
            .components
            .iter()
            .flat_map(|c| c.documents.iter().map(|d| d.artifact.clone())),
    );
    let manifest = output.join("manifest.json");
    records::create_json(&manifest, &result)?;
    batch::verify(&root, &records::artifact(&root, &manifest)?)?;
    batch::register(&root, &manifest)?;
    println!(
        "Registry license evidence archived at {}. Native/system and source/distribution reviews remain pending.",
        manifest.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_nested_and_compound_license_documents() {
        for name in [
            "LICENSE-MIT",
            "license-apache-2.0",
            "COPYING",
            "UNLICENSE",
            "licenses/subproject.txt",
            "src/NOTICE.md",
        ] {
            assert!(document_path(name, DocumentPolicy::TextDocuments), "{name}");
        }
        for name in [
            "src/lib.rs",
            "test/source.rs",
            "Cargo.toml",
            "src/copying.rs",
            "src/tests/copying.rs",
            "src/notices_missing.rs",
        ] {
            assert!(
                !document_path(name, DocumentPolicy::TextDocuments),
                "{name}"
            );
        }
        assert!(document_path(
            "src/copying.rs",
            DocumentPolicy::LegacyPrefix
        ));
        let old = serde_json::json!({"layout_version":1,"parent_batch":{"path":"parent.json","sha256":"0".repeat(64)},"target":"test","components":[],"remaining":["review pending"]});
        let old: Inventory = serde_json::from_value(old).unwrap();
        assert!(matches!(old.document_policy, DocumentPolicy::LegacyPrefix));
    }
    #[test]
    fn uses_only_resolved_packages_and_rejects_missing_or_duplicate_nodes() {
        let fixture = |nodes| {
            serde_json::from_value::<Metadata>(serde_json::json!({"packages":[{"id":"a","name":"a","version":"1.0.0","source":"registry+test","license":"MIT","license_file":null},{"id":"unused","name":"unused","version":"1.0.0","source":"registry+test","license":null,"license_file":null}],"resolve":{"nodes":nodes}})).unwrap()
        };
        assert_eq!(
            selected(&fixture(serde_json::json!([{"id":"a"}])))
                .unwrap()
                .len(),
            1
        );
        assert!(selected(&fixture(serde_json::json!([{"id":"missing"}]))).is_err());
        assert!(selected(&fixture(serde_json::json!([{"id":"a"},{"id":"a"}]))).is_err());
    }

    #[test]
    fn rejects_missing_documents_changed_declarations_and_replaced_vendor_evidence() {
        struct Fixture(std::path::PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let base = env::temp_dir().canonicalize().unwrap();
                let path = self.0.canonicalize().unwrap();
                assert!(path.starts_with(&base) && path != base);
                fs::remove_dir_all(path).unwrap();
            }
        }
        let fixture = Fixture(env::temp_dir().join(format!(
            "mfs-licenses-{}-{}", std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        )));
        let root = &fixture.0;
        let input = root.join("runs/parent/inputs");
        let vendor = input.join("vendor/a-1.0.0");
        fs::create_dir_all(&vendor).unwrap();
        let license = vendor.join("LICENSE-MIT");
        fs::write(&license, "original license evidence").unwrap();
        let checksum = vendor.join(".cargo-checksum.json");
        records::create_json(&checksum, &serde_json::json!({"package":"0".repeat(64),"files":{"LICENSE-MIT": records::hash(&license).unwrap()}})).unwrap();
        let metadata = input.join("cargo-metadata.json");
        records::create_json(&metadata, &serde_json::json!({"packages":[{"id":"a","name":"a","version":"1.0.0","source":"registry+test","license":"MIT","license_file":null}],"resolve":{"nodes":[{"id":"a"}]}})).unwrap();
        let target = input.join("dependency-target.txt");
        fs::write(&target, "test-target\n").unwrap();
        let lock = input.join("Cargo.lock");
        fs::write(&lock, "archived test lock").unwrap();
        let parent = root.join("runs/parent/manifest.json");
        let run = batch::Run {
            layout_version: records::LAYOUT_VERSION,
            run_id: "parent".into(),
            phase: "P0".into(),
            phase_status: "incomplete".into(),
            checks: vec![],
            pending: vec![],
            inputs: [metadata, target, lock, checksum.clone(), license.clone()]
                .iter()
                .map(|p| records::artifact(root, p).unwrap())
                .collect(),
        };
        records::create_json(&parent, &run).unwrap();
        let copy = root.join("copy.txt");
        fs::copy(&license, &copy).unwrap();
        let mut inventory = Inventory {
            layout_version: records::LAYOUT_VERSION,
            parent_batch: records::artifact(root, &parent).unwrap(),
            target: "test-target".into(),
            document_policy: DocumentPolicy::TextDocuments,
            components: vec![Component {
                package_id: "a".into(),
                name: "a".into(),
                version: "1.0.0".into(),
                source: "registry+test".into(),
                declared_license: Some("MIT".into()),
                declared_license_file: None,
                registry_checksum: "0".repeat(64),
                documents: vec![Document {
                    original_path: "LICENSE-MIT".into(),
                    artifact: records::artifact(root, &copy).unwrap(),
                }],
            }],
            remaining: vec!["Source and distribution review pending".into()],
        };
        assert!(validate(root, &inventory).is_ok());
        let document = inventory.components[0].documents.pop().unwrap();
        assert!(validate(root, &inventory).is_err());
        inventory.components[0].documents.push(document);
        inventory.components[0].declared_license = Some("Apache-2.0".into());
        assert!(validate(root, &inventory).is_err());
        inventory.components[0].declared_license = Some("MIT".into());
        fs::write(&license, "replacement license evidence").unwrap();
        fs::copy(&license, &copy).unwrap();
        inventory.components[0].documents[0].artifact = records::artifact(root, &copy).unwrap();
        assert!(validate(root, &inventory).is_err());
        fs::write(&checksum, serde_json::to_vec(&serde_json::json!({"package":"0".repeat(64),"files":{"LICENSE-MIT": records::hash(&license).unwrap()}})).unwrap()).unwrap();
        assert!(validate(root, &inventory).is_err());
        let mut claim = serde_json::to_value(&inventory).unwrap();
        claim["distribution_approved"] = true.into();
        assert!(serde_json::from_value::<Inventory>(claim).is_err());
    }
}
