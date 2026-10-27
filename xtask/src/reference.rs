// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use std::{collections::BTreeSet, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    Result, git,
    records::{self, Artifact, Index, LAYOUT_VERSION},
};

const URL: &str = "https://github.com/MaaXYZ/MaaFramework.git";
const BASE: &str = "0fc6025c2fc224a171b57d89505e8d47acd2a8b1";
const HEAD: &str = "f603a04633b0e43a8bbcb94ddef60fc349ad29a1";
const COMMITS: [&str; 2] = ["54b116aa89f70e5ae156c05120cabaddca7702a8", HEAD];
const PATCH_FILE: &str = "source/MaaWin32ControlUnit/Screencap/HwndUtils.hpp";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Baseline {
    layout_version: u32,
    kind: String,
    baseline_id: String,
    repository_url: String,
    commit: String,
    tree: String,
    status: String,
    source_archive: Artifact,
    sources: Vec<Source>,
    inventory: Artifact,
    license_audit: Artifact,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Source {
    original_path: String,
    git_blob: String,
    artifact: Artifact,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Patch {
    layout_version: u32,
    pull_request: String,
    base: String,
    head: String,
    commits: Vec<String>,
    files: Vec<String>,
    patch: Artifact,
    head_sources: Vec<Source>,
    parent_u: Artifact,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    layout_version: u32,
    status: String,
    entries: Vec<Responsibility>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Responsibility {
    source_path: String,
    area: String,
    capture_evidence: Vec<Evidence>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    line: usize,
    text: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LicenseAudit {
    layout_version: u32,
    status: String,
    upstream_license: Artifact,
    file_statements: Vec<Evidence>,
    notes: Vec<String>,
}

pub fn prepare(path: &Path) -> Result<()> {
    let root = records::external_root(path)?;
    let index_path = root.join("manifest.json");
    let mut index = if index_path.exists() {
        let existing = records::read::<Index>(&index_path)?;
        records::version(existing.layout_version)?;
        if !existing.baselines.is_empty() || !existing.patches.is_empty() {
            verify(&root)?;
        }
        existing
    } else {
        Index {
            layout_version: LAYOUT_VERSION,
            ..Index::default()
        }
    };
    if !index.baselines.is_empty() || !index.patches.is_empty() {
        println!(
            "Fixed preparation inputs already exist and passed verification. No baseline was changed."
        );
        return Ok(());
    }

    let identity = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let baseline_id = format!("maafw-0fc6025-preparation-{identity}");
    let baseline = root.join("baselines/u").join(&baseline_id);
    // A partially prepared directory is evidence, too. Leave it intact on failure.
    // A caller can inspect it and use a fresh external root rather than overwriting it.
    fs::create_dir_all(baseline.parent().unwrap())?;
    fs::create_dir(&baseline)?;
    let reference = baseline.join("reference");
    fs::create_dir(&reference)?;
    let upstream = reference.join("upstream");
    fs::create_dir(&upstream)?;
    println!("Fetching fixed upstream {BASE} and PR head {HEAD} ...");
    git(&upstream, &["init", "--quiet"])?;
    git(&upstream, &["remote", "add", "origin", URL])?;
    git(
        &upstream,
        &["fetch", "--no-tags", "--depth=3", "origin", HEAD],
    )?;
    if text(git(&upstream, &["rev-parse", "FETCH_HEAD"])?)? != HEAD {
        return Err("fetched PR head identity does not match the plan".into());
    }
    if text(git(
        &upstream,
        &["rev-parse", &format!("{BASE}^{{commit}}")],
    )?)? != BASE
    {
        return Err("fixed upstream is not present in the PR ancestry".into());
    }
    let commits = text(git(
        &upstream,
        &["rev-list", "--reverse", &format!("{BASE}..{HEAD}")],
    )?)?;
    if commits.lines().collect::<Vec<_>>() != COMMITS {
        return Err("PR cumulative range does not contain exactly the planned two commits".into());
    }
    git(
        &upstream,
        &[
            "-c",
            "core.autocrlf=false",
            "checkout",
            "--quiet",
            "--detach",
            BASE,
        ],
    )?;
    let tree = text(git(&upstream, &["rev-parse", "HEAD^{tree}"])?)?;
    let archive = reference.join("upstream.tar");
    git(
        &upstream,
        &[
            "archive",
            "--format=tar",
            "--output",
            archive.to_str().ok_or("archive path is not UTF-8")?,
            BASE,
        ],
    )?;
    let names = tracked_files(&upstream)?;
    let mut sources = Vec::new();
    let mut entries = Vec::new();
    let mut statements = Vec::new();
    for name in names
        .iter()
        .map(String::as_str)
        .filter(|name| relevant(name))
    {
        let path = upstream.join(name);
        let bytes = fs::read(&path)?;
        let content = String::from_utf8_lossy(&bytes);
        let evidence = content
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                [
                    "screencap",
                    "screenshot",
                    "Screencap",
                    "ScreenCapture",
                    "cv::",
                    "wait_",
                    "WaitFor",
                    "join(",
                    "stop(",
                    "GetBitmapBits",
                    "GetDIBits",
                ]
                .iter()
                .any(|term| line.contains(term))
            })
            .map(|(line, value)| Evidence {
                line: line + 1,
                text: value.to_owned(),
            })
            .collect();
        for (line, value) in content.lines().enumerate().filter(|(_, line)| {
            ["Copyright", "SPDX", "LGPL", "GNU Lesser", "GNU General"]
                .iter()
                .any(|term| line.contains(term))
        }) {
            statements.push(Evidence {
                line: line + 1,
                text: format!("{name}: {value}"),
            });
        }
        entries.push(Responsibility {
            source_path: name.to_owned(),
            area: area(name).to_owned(),
            capture_evidence: evidence,
        });
        sources.push(Source {
            original_path: name.to_owned(),
            git_blob: text(git(&upstream, &["rev-parse", &format!("{BASE}:{name}")])?)?,
            artifact: records::artifact(&root, &path)?,
        });
    }
    let mappings = baseline.join("mappings");
    fs::create_dir(&mappings)?;
    let inventory_path = mappings.join("source-inventory.json");
    records::create_json(&inventory_path, &Inventory {
        layout_version: LAYOUT_VERSION,
        status: "source discovery; responsibility-by-responsibility review and parity matrix pending".into(),
        entries,
    })?;
    let audits = baseline.join("audits");
    fs::create_dir(&audits)?;
    let audit_path = audits.join("license.json");
    records::create_json(&audit_path, &LicenseAudit {
        layout_version: LAYOUT_VERSION,
        status: "upstream license and file statements archived; dependency and distribution review pending".into(),
        upstream_license: records::artifact(&root, &upstream.join("LICENSE.md"))?,
        file_statements: statements,
        notes: vec![
            "Source archive preserves the complete fixed upstream tree, including original notices.".into(),
            "No extracted capture implementation has entered the product yet.".into(),
            "The PR source is from the same repository; its exact head file and cumulative diff are separately archived.".into(),
            "Dependency licenses must be audited against the actual per-run dependency resolution.".into(),
        ],
    })?;
    let baseline_path = baseline.join("manifest.json");
    records::create_json(
        &baseline_path,
        &Baseline {
            layout_version: LAYOUT_VERSION,
            kind: "u".into(),
            baseline_id,
            repository_url: URL.into(),
            commit: BASE.into(),
            tree,
            status:
                "P0 source preparation; reference executable, Rust snapshot and acceptance pending"
                    .into(),
            source_archive: records::artifact(&root, &archive)?,
            sources,
            inventory: records::artifact(&root, &inventory_path)?,
            license_audit: records::artifact(&root, &audit_path)?,
        },
    )?;
    index
        .baselines
        .push(records::artifact(&root, &baseline_path)?);

    let patch_directory = root.join("patches/pr-1587");
    fs::create_dir_all(patch_directory.parent().unwrap())?;
    fs::create_dir(&patch_directory)?;
    let patch_path = patch_directory.join("1587.patch");
    let range = format!("{BASE}..{HEAD}");
    let files = text(git(&upstream, &["diff", "--name-only", &range])?)?;
    if files.lines().collect::<Vec<_>>() != [PATCH_FILE] {
        return Err("PR changed files differ from the fixed scope".into());
    }
    fs::write(
        &patch_path,
        git(
            &upstream,
            &[
                "diff",
                "--binary",
                "--full-index",
                "--no-ext-diff",
                "--no-textconv",
                &range,
            ],
        )?,
    )?;
    git(
        &upstream,
        &[
            "apply",
            "--check",
            patch_path.to_str().ok_or("patch path is not UTF-8")?,
        ],
    )?;
    let head_path = patch_directory.join("HwndUtils.head.hpp");
    fs::write(
        &head_path,
        git(&upstream, &["show", &format!("{HEAD}:{PATCH_FILE}")])?,
    )?;
    let patch_manifest = patch_directory.join("manifest.json");
    records::create_json(
        &patch_manifest,
        &Patch {
            layout_version: LAYOUT_VERSION,
            pull_request: "https://github.com/MaaXYZ/MaaFramework/pull/1587".into(),
            base: BASE.into(),
            head: HEAD.into(),
            commits: COMMITS.iter().map(|s| (*s).to_owned()).collect(),
            files: vec![PATCH_FILE.into()],
            patch: records::artifact(&root, &patch_path)?,
            head_sources: vec![Source {
                original_path: PATCH_FILE.into(),
                git_blob: text(git(
                    &upstream,
                    &["rev-parse", &format!("{HEAD}:{PATCH_FILE}")],
                )?)?,
                artifact: records::artifact(&root, &head_path)?,
            }],
            parent_u: index.baselines[0].clone(),
        },
    )?;
    index
        .patches
        .push(records::artifact(&root, &patch_manifest)?);
    records::create_json(&index_path, &index)?;
    verify(&root)
}

pub fn verify(path: &Path) -> Result<()> {
    let root = records::external_root(path)?;
    let index: Index = records::read(&root.join("manifest.json"))?;
    records::version(index.layout_version)?;
    if index.baselines.len() != 1 || index.patches.len() != 1 {
        return Err("P0 index requires one fixed U preparation baseline and one PR patch".into());
    }
    let mut paths = BTreeSet::new();
    for artifact in index
        .baselines
        .iter()
        .chain(&index.patches)
        .chain(&index.runs)
    {
        if !paths.insert(&artifact.path) {
            return Err("duplicate index path".into());
        }
        records::verify(&root, artifact)?;
    }
    for artifact in &index.runs {
        crate::batch::verify(&root, artifact)?;
    }
    let baseline_path = records::verify(&root, &index.baselines[0])?;
    let baseline: Baseline = records::read(&baseline_path)?;
    records::version(baseline.layout_version)?;
    if baseline.kind != "u"
        || baseline.commit != BASE
        || baseline.repository_url != URL
        || !baseline
            .baseline_id
            .starts_with("maafw-0fc6025-preparation-")
    {
        return Err("unexpected U source identity".into());
    }
    records::verify(&root, &baseline.source_archive)?;
    let upstream = baseline_path.parent().unwrap().join("reference/upstream");
    if text(git(&upstream, &["rev-parse", "HEAD"])?)? != BASE
        || text(git(&upstream, &["rev-parse", "HEAD^{tree}"])?)? != baseline.tree
    {
        return Err("upstream checkout does not match the fixed U tree".into());
    }
    if !git(
        &upstream,
        &["status", "--porcelain", "--untracked-files=all"],
    )?
    .is_empty()
    {
        return Err("upstream checkout contains modified or untracked inputs".into());
    }
    let expected = tracked_files(&upstream)?;
    let expected: BTreeSet<_> = expected
        .iter()
        .map(String::as_str)
        .filter(|name| relevant(name))
        .collect();
    let actual: BTreeSet<_> = baseline
        .sources
        .iter()
        .map(|s| s.original_path.as_str())
        .collect();
    if expected != actual || actual.len() != baseline.sources.len() {
        return Err("source inventory is incomplete or duplicated".into());
    }
    for source in &baseline.sources {
        verify_source(&root, &upstream, BASE, source)?;
    }
    let inventory: Inventory = records::read(&records::verify(&root, &baseline.inventory)?)?;
    records::version(inventory.layout_version)?;
    if inventory
        .entries
        .iter()
        .map(|e| e.source_path.as_str())
        .collect::<BTreeSet<_>>()
        != expected
        || inventory.entries.len() != expected.len()
    {
        return Err("responsibility inventory does not cover the selected sources exactly".into());
    }
    let audit: LicenseAudit = records::read(&records::verify(&root, &baseline.license_audit)?)?;
    records::version(audit.layout_version)?;
    records::verify(&root, &audit.upstream_license)?;
    let patch: Patch = records::read(&records::verify(&root, &index.patches[0])?)?;
    records::version(patch.layout_version)?;
    if patch.base != BASE
        || patch.head != HEAD
        || patch.commits != COMMITS
        || patch.files != [PATCH_FILE]
        || patch.head_sources.len() != 1
        || patch.head_sources[0].original_path != PATCH_FILE
        || patch.parent_u.path != index.baselines[0].path
        || patch.parent_u.sha256 != index.baselines[0].sha256
        || patch.pull_request != "https://github.com/MaaXYZ/MaaFramework/pull/1587"
    {
        return Err("fixed PR identity, range or parent U does not match".into());
    }
    let patch_path = records::verify(&root, &patch.patch)?;
    if fs::read(&patch_path)?
        != git(
            &upstream,
            &[
                "diff",
                "--binary",
                "--full-index",
                "--no-ext-diff",
                "--no-textconv",
                &format!("{BASE}..{HEAD}"),
            ],
        )?
    {
        return Err("patch does not equal the fixed cumulative Git diff".into());
    }
    for source in &patch.head_sources {
        verify_source(&root, &upstream, HEAD, source)?;
    }
    println!(
        "Verified {} source files, full upstream archive, inventory, license record and cumulative PR patch.\nP0 preparation only; native capture and phase acceptance have not been asserted.",
        baseline.sources.len()
    );
    Ok(())
}

fn verify_source(root: &Path, upstream: &Path, commit: &str, source: &Source) -> Result<()> {
    let path = records::verify(root, &source.artifact)?;
    let object = format!("{commit}:{}", source.original_path);
    if text(git(upstream, &["rev-parse", &object])?)? != source.git_blob
        || fs::read(path)? != git(upstream, &["show", &object])?
    {
        return Err(format!("source differs from Git object: {}", source.original_path).into());
    }
    Ok(())
}

pub(crate) struct ReviewContext {
    pub(crate) upstream: std::path::PathBuf,
    pub(crate) commit: String,
    pub(crate) required_sources: BTreeSet<String>,
}

pub(crate) fn review_context(root: &Path, parent: &Artifact) -> Result<ReviewContext> {
    let index: Index = records::read(&root.join("manifest.json"))?;
    records::version(index.layout_version)?;
    if !index
        .baselines
        .iter()
        .any(|entry| entry.path == parent.path && entry.sha256 == parent.sha256)
    {
        return Err("catalog parent is not an indexed fixed U preparation baseline".into());
    }
    let path = records::verify(root, parent)?;
    let baseline: Baseline = records::read(&path)?;
    records::version(baseline.layout_version)?;
    if baseline.kind != "u" || baseline.commit != BASE || baseline.repository_url != URL {
        return Err("catalog requires the fixed unpatched upstream".into());
    }
    let upstream = path.parent().unwrap().join("reference/upstream");
    if text(git(&upstream, &["rev-parse", "HEAD"])?)? != BASE
        || text(git(&upstream, &["rev-parse", "HEAD^{tree}"])?)? != baseline.tree
    {
        return Err("catalog checkout differs from its fixed parent".into());
    }
    // The immutable discovery baseline predates the complete public-header and
    // portal handoff review. Extend review coverage without rewriting it.
    let mut required_sources: BTreeSet<_> = baseline
        .sources
        .into_iter()
        .map(|s| s.original_path)
        .collect();
    required_sources.extend(tracked_files(&upstream)?.into_iter().filter(|path| {
        path.starts_with("include/MaaControlUnit/")
            || path.starts_with("source/MaaToolkit/Portal/")
            || path.starts_with("include/MaaToolkit/Portal/")
            || [
                "include/MaaFramework/MaaDef.h",
                "include/MaaFramework/MaaPort.h",
                "include/MaaFramework/Instance/MaaCustomController.h",
            ]
            .contains(&path.as_str())
    }));
    Ok(ReviewContext {
        upstream,
        commit: baseline.commit,
        required_sources,
    })
}

fn text(bytes: Vec<u8>) -> Result<String> {
    Ok(String::from_utf8(bytes)?.trim().to_owned())
}

fn tracked_files(upstream: &Path) -> Result<Vec<String>> {
    Ok(text(git(
        upstream,
        &["ls-tree", "-r", "--format=%(objecttype) %(path)", BASE],
    )?)?
    .lines()
    .filter_map(|line| line.strip_prefix("blob ").map(str::to_owned))
    .collect())
}

fn relevant(path: &str) -> bool {
    path.starts_with("source/MaaWin32ControlUnit/") && !path.contains("/Input/")
        || path.starts_with("source/MaaLinuxControlUnit/") && !path.contains("/Input/")
        || path.starts_with("source/MaaMacOSControlUnit/") && !path.contains("/Input/")
        || path.starts_with("source/MaaCustomControlUnit/")
        || path.starts_with("source/ControlUnit/CustomControlUnit/")
        || path.starts_with("include/ControlUnit/")
        || path.starts_with("3rdparty/")
        || path.contains("wayland")
        || path.contains("wlr-screencopy")
        || path == "LICENSE.md"
        || path == ".gitmodules"
        || path == "CMakeLists.txt"
}

fn area(path: &str) -> &str {
    if path.contains("/Screencap/") {
        "backend and direct helper"
    } else if path.contains("/Manager/") || path.contains("CustomControl") {
        "construction, selection and capture call chain"
    } else if path.contains("/Base/") {
        "platform context, foreground, shared memory or native lifecycle"
    } else if path.contains("wayland") || path.contains("wlr-screencopy") {
        "Wayland client/protocol input"
    } else {
        "interface, source/license or build input"
    }
}
