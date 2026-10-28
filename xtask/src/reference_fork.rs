// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Explicit corrected-fork revisions alongside immutable historical records.
use super::*;
use std::{
    env,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const FORK_URL: &str = "https://github.com/AbyssRei/MaaFramework.git";
const POLICY: &str = "exact-client-screen-rect-v1";
const TEST_ROOT: &str = "test/win32_test/window_geometry/";
const FORK_FILES: [&str; 4] = [
    PATCH_FILE,
    "test/win32_test/window_geometry/CMakeLists.txt",
    "test/win32_test/window_geometry/logging/MaaUtils/Logger.h",
    "test/win32_test/window_geometry/window_geometry_test.cpp",
];
const STATUS: &str =
    "F source preparation only; full extraction/native acceptance and source freeze remain pending";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ForkBaseline {
    layout_version: u32,
    kind: String,
    baseline_id: String,
    revision: String,
    policy: String,
    repository_url: String,
    ancestor: String,
    commit: String,
    tree: String,
    status: String,
    parent_ancestor: Artifact,
    patch: Artifact,
    source_archive: Artifact,
    sources: Vec<Source>,
    inventory: Artifact,
    license_audit: Artifact,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ForkPatch {
    layout_version: u32,
    revision: String,
    policy: String,
    pull_request: String,
    repository_url: String,
    ancestor: String,
    head: String,
    head_tree: String,
    commits: Vec<String>,
    files: Vec<String>,
    patch: Artifact,
    head_sources: Vec<Source>,
    parent_ancestor: Artifact,
}
fn identity(revision: &str, head: &str) -> Result<()> {
    if revision.is_empty()
        || revision.len() > 96
        || !revision
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("fork revision must be a short lowercase identifier".into());
    }
    if head.len() != 40
        || !head
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || head == BASE
        || head == HEAD
        || head.bytes().all(|b| b == b'0')
    {
        return Err("fork head must be an exact corrected commit, not the ancestor, old PR head or a branch".into());
    }
    Ok(())
}
fn scope(files: &[String]) -> Result<()> {
    if files.iter().map(String::as_str).collect::<Vec<_>>() != FORK_FILES {
        return Err("corrected fork range must contain only the window helper and its explicit regression tests".into());
    }
    Ok(())
}
fn same(left: &Artifact, right: &Artifact) -> bool {
    left.path == right.path && left.sha256 == right.sha256
}
fn files_at(upstream: &Path, head: &str) -> Result<Vec<String>> {
    Ok(text(git(
        upstream,
        &["ls-tree", "-r", "--format=%(objecttype) %(path)", head],
    )?)?
    .lines()
    .filter_map(|line| line.strip_prefix("blob ").map(str::to_owned))
    .collect())
}
fn included(path: &str) -> bool {
    relevant(path) || path.starts_with(TEST_ROOT) || path == "test/win32_test/CMakeLists.txt"
}
fn range(upstream: &Path, head: &str, operation: &str) -> Result<Vec<u8>> {
    let range = format!("{BASE}..{head}");
    match operation {
        "patch" => git(
            upstream,
            &[
                "diff",
                "--binary",
                "--full-index",
                "--no-ext-diff",
                "--no-textconv",
                &range,
            ],
        ),
        "files" => git(
            upstream,
            &[
                "diff",
                "--name-only",
                "--no-ext-diff",
                "--no-textconv",
                &range,
            ],
        ),
        "commits" => git(upstream, &["rev-list", "--reverse", &range]),
        _ => Err("unknown fork source query".into()),
    }
}
fn source(root: &Path, upstream: &Path, head: &str, path: &str) -> Result<Source> {
    Ok(Source {
        original_path: path.into(),
        git_blob: text(git(upstream, &["rev-parse", &format!("{head}:{path}")])?)?,
        artifact: records::artifact(root, &upstream.join(path))?,
    })
}
pub(crate) fn selected(root: &Path, index: &Index) -> Result<Artifact> {
    let selection = index.selected_baseline.as_ref().ok_or(
        "no corrected fork selected; run reference prepare-fork with its exact head and revision",
    )?;
    if !selection.path.starts_with("baselines/f/")
        || !index.baselines.iter().any(|entry| same(entry, selection))
    {
        return Err("selected fork is not an indexed immutable F baseline".into());
    }
    let baseline: ForkBaseline = records::read(&records::verify(root, selection)?)?;
    identity(&baseline.revision, &baseline.commit)?;
    if baseline.kind != "f"
        || baseline.policy != POLICY
        || baseline.repository_url != FORK_URL
        || baseline.ancestor != BASE
    {
        return Err("selected fork identity or policy differs".into());
    }
    Ok(selection.clone())
}
pub(crate) fn prepare(args: &[String]) -> Result<()> {
    let mut root: Option<PathBuf> = None;
    let (mut revision, mut head) = (None, None);
    let mut args = args.iter();
    while let Some(option) = args.next() {
        let value = args
            .next()
            .ok_or("fork preparation option requires a value")?;
        match option.as_str() {
            "--root" if root.is_none() => root = Some(value.into()),
            "--revision" if revision.is_none() => revision = Some(value.as_str()),
            "--head" if head.is_none() => head = Some(value.as_str()),
            _ => return Err(
                "expected unique --revision <id> --head <exact SHA> [--root <external directory>]"
                    .into(),
            ),
        }
    }
    let revision = revision.ok_or("explicit fork revision required")?;
    let head = head.ok_or("explicit corrected fork head required")?;
    identity(revision, head)?;
    let root = records::external_root(
        &root
            .or_else(|| env::var_os("MFS_REFERENCE_ROOT").map(Into::into))
            .unwrap_or_else(|| {
                crate::repository()
                    .parent()
                    .unwrap()
                    .join("maafw-screencap-reference")
            }),
    )?;
    // Preparation validates source records; `reference verify` separately
    // checks historical run payloads without relabelling them as F results.
    super::verify_sources(&root)?;
    let index_path = root.join("manifest.json");
    let index_hash = records::hash(&index_path)?;
    let mut index: Index = records::read(&index_path)?;
    for entry in index
        .baselines
        .iter()
        .filter(|entry| entry.path.starts_with("baselines/f/"))
    {
        let existing: ForkBaseline = records::read(&records::verify(&root, entry)?)?;
        if existing.revision == revision {
            if existing.commit != head {
                return Err("fork revision is immutable; use a new revision identifier for a different head".into());
            }
            if !index
                .selected_baseline
                .as_ref()
                .is_some_and(|selection| same(selection, entry))
            {
                index.selected_baseline = Some(entry.clone());
                replace_index(&root, &index, &index_hash)?;
            }
            println!(
                "Selected verified immutable F revision {revision} at {head}; no archived baseline was modified."
            );
            return Ok(());
        }
    }
    let parent = index
        .baselines
        .iter()
        .find(|entry| entry.path.starts_with("baselines/u/"))
        .ok_or("historical ancestor baseline missing")?
        .clone();
    let baseline_id = format!(
        "{revision}-{}-{}",
        &head[..12],
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let directory = root.join("baselines/f").join(&baseline_id);
    fs::create_dir_all(directory.parent().unwrap())?;
    fs::create_dir(&directory)?;
    let reference = directory.join("reference");
    fs::create_dir(&reference)?;
    let upstream = reference.join("upstream");
    fs::create_dir(&upstream)?;
    // Fetch an exact object, never a moving branch or the old PR head.
    git(&upstream, &["init", "--quiet"])?;
    git(&upstream, &["remote", "add", "origin", FORK_URL])?;
    git(&upstream, &["fetch", "--no-tags", "origin", head])?;
    if text(git(&upstream, &["rev-parse", "FETCH_HEAD"])?)? != head {
        return Err("fetched corrected fork differs from requested head".into());
    }
    git(&upstream, &["merge-base", "--is-ancestor", BASE, head])?;
    git(
        &upstream,
        &[
            "-c",
            "core.autocrlf=false",
            "checkout",
            "--quiet",
            "--detach",
            head,
        ],
    )?;
    let tree = text(git(&upstream, &["rev-parse", "HEAD^{tree}"])?)?;
    let changed: Vec<String> = text(range(&upstream, head, "files")?)?
        .lines()
        .map(str::to_owned)
        .collect();
    scope(&changed)?;
    let commits = text(range(&upstream, head, "commits")?)?
        .lines()
        .map(str::to_owned)
        .collect();
    let archive = reference.join("upstream.tar");
    git(
        &upstream,
        &[
            "archive",
            "--format=tar",
            "--output",
            archive.to_str().ok_or("archive path is not UTF-8")?,
            head,
        ],
    )?;
    let mut sources = Vec::new();
    let mut entries = Vec::new();
    let mut statements = Vec::new();
    for path in files_at(&upstream, head)?
        .into_iter()
        .filter(|path| included(path))
    {
        let bytes = fs::read(upstream.join(&path))?;
        let content = String::from_utf8_lossy(&bytes);
        let evidence = content
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                [
                    "screencap",
                    "Screencap",
                    "ScreenCapture",
                    "cv::",
                    "ensure_window_on_screen",
                    "wait_",
                    "join(",
                    "stop(",
                ]
                .iter()
                .any(|term| line.contains(term))
            })
            .map(|(i, text)| Evidence {
                line: i + 1,
                text: text.into(),
            })
            .collect();
        for (i, line) in content.lines().enumerate().filter(|(_, line)| {
            ["Copyright", "SPDX", "LGPL", "GNU Lesser", "GNU General"]
                .iter()
                .any(|term| line.contains(term))
        }) {
            statements.push(Evidence {
                line: i + 1,
                text: format!("{path}: {line}"),
            });
        }
        entries.push(Responsibility {
            source_path: path.clone(),
            area: area(&path).into(),
            capture_evidence: evidence,
        });
        sources.push(source(&root, &upstream, head, &path)?);
    }
    let mappings = directory.join("mappings");
    fs::create_dir(&mappings)?;
    let inventory = mappings.join("source-inventory.json");
    records::create_json(&inventory, &Inventory {layout_version:LAYOUT_VERSION,status:"Corrected fork source discovery; full migration, Rust snapshot and native acceptance pending".into(),entries})?;
    let audits = directory.join("audits");
    fs::create_dir(&audits)?;
    let audit = audits.join("license.json");
    records::create_json(&audit, &LicenseAudit {layout_version:LAYOUT_VERSION,status:"Fork source licence and file statements archived; distribution/dependency review remains per build".into(),upstream_license:records::artifact(&root,&upstream.join("LICENSE.md"))?,file_statements:statements,notes:vec!["Complete exact fork tree and original notices are retained in the source archive.".into(),"Historical U/P sources, patches, native failures and acceptance boundaries retain their original identities.".into(),"Source preparation is not full F or SDK acceptance.".into()]})?;
    let patches = root.join("patches/pr-1587").join(revision);
    fs::create_dir(&patches)?;
    let patch_file = patches.join("1587.patch");
    fs::write(&patch_file, range(&upstream, head, "patch")?)?;
    let patch_manifest = patches.join("manifest.json");
    records::create_json(
        &patch_manifest,
        &ForkPatch {
            layout_version: LAYOUT_VERSION,
            revision: revision.into(),
            policy: POLICY.into(),
            pull_request: "https://github.com/MaaXYZ/MaaFramework/pull/1587".into(),
            repository_url: FORK_URL.into(),
            ancestor: BASE.into(),
            head: head.into(),
            head_tree: tree.clone(),
            commits,
            files: changed.clone(),
            patch: records::artifact(&root, &patch_file)?,
            head_sources: changed
                .iter()
                .map(|path| source(&root, &upstream, head, path))
                .collect::<Result<_>>()?,
            parent_ancestor: parent.clone(),
        },
    )?;
    let patch = records::artifact(&root, &patch_manifest)?;
    let baseline_path = directory.join("manifest.json");
    records::create_json(
        &baseline_path,
        &ForkBaseline {
            layout_version: LAYOUT_VERSION,
            kind: "f".into(),
            baseline_id,
            revision: revision.into(),
            policy: POLICY.into(),
            repository_url: FORK_URL.into(),
            ancestor: BASE.into(),
            commit: head.into(),
            tree,
            status: STATUS.into(),
            parent_ancestor: parent.clone(),
            patch: patch.clone(),
            source_archive: records::artifact(&root, &archive)?,
            sources,
            inventory: records::artifact(&root, &inventory)?,
            license_audit: records::artifact(&root, &audit)?,
        },
    )?;
    let baseline = records::artifact(&root, &baseline_path)?;
    index.baselines.push(baseline.clone());
    index.patches.push(patch);
    index.selected_baseline = Some(baseline);
    verify_all(&root, &index, &parent)?;
    replace_index(&root, &index, &index_hash)?;
    println!(
        "Prepared F revision={revision} head={head}; historical records preserved; full_F_acceptance=0"
    );
    Ok(())
}
fn replace_index(root: &Path, index: &Index, before: &str) -> Result<()> {
    use std::io::Write;
    let target = root.join("manifest.json");
    if records::hash(&target)? != before {
        return Err(
            "reference index changed during fork preparation; unregistered inputs retained".into(),
        );
    }
    let temporary = root.join(format!("manifest-fork-next-{}.json", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(index)?)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, target)?;
    Ok(())
}
pub(super) fn verify_all(root: &Path, index: &Index, parent: &Artifact) -> Result<()> {
    let mut patch_paths = BTreeSet::new();
    for entry in index
        .baselines
        .iter()
        .filter(|entry| !entry.path.starts_with("baselines/u/"))
    {
        if !entry.path.starts_with("baselines/f/") {
            return Err("unknown baseline kind".into());
        }
        let path = records::verify(root, entry)?;
        let baseline: ForkBaseline = records::read(&path)?;
        records::version(baseline.layout_version)?;
        identity(&baseline.revision, &baseline.commit)?;
        if baseline.kind != "f"
            || baseline.status != STATUS
            || baseline.policy != POLICY
            || baseline.repository_url != FORK_URL
            || baseline.ancestor != BASE
            || !same(&baseline.parent_ancestor, parent)
            || entry.path != format!("baselines/f/{}/manifest.json", baseline.baseline_id)
            || !index
                .patches
                .iter()
                .any(|patch| same(patch, &baseline.patch))
            || !patch_paths.insert(baseline.patch.path.clone())
        {
            return Err("F source identity, parent or patch association differs".into());
        }
        let upstream = path.parent().unwrap().join("reference/upstream");
        if text(git(&upstream, &["rev-parse", "HEAD"])?)? != baseline.commit
            || text(git(&upstream, &["rev-parse", "HEAD^{tree}"])?)? != baseline.tree
            || !git(
                &upstream,
                &["status", "--porcelain", "--untracked-files=all"],
            )?
            .is_empty()
        {
            return Err("F checkout differs from its immutable commit/tree".into());
        }
        git(
            &upstream,
            &["merge-base", "--is-ancestor", BASE, &baseline.commit],
        )?;
        if fs::read(records::verify(root, &baseline.source_archive)?)?
            != git(&upstream, &["archive", "--format=tar", &baseline.commit])?
        {
            return Err("F source archive differs from its complete Git tree".into());
        }
        let expected: BTreeSet<_> = files_at(&upstream, &baseline.commit)?
            .into_iter()
            .filter(|path| included(path))
            .collect();
        let actual: BTreeSet<_> = baseline
            .sources
            .iter()
            .map(|source| source.original_path.clone())
            .collect();
        if actual != expected || actual.len() != baseline.sources.len() {
            return Err("F source inventory is incomplete or duplicated".into());
        }
        for source in &baseline.sources {
            verify_source(root, &upstream, &baseline.commit, source)?;
        }
        let inventory: Inventory = records::read(&records::verify(root, &baseline.inventory)?)?;
        records::version(inventory.layout_version)?;
        if inventory.entries.len() != expected.len()
            || inventory
                .entries
                .iter()
                .map(|entry| entry.source_path.clone())
                .collect::<BTreeSet<_>>()
                != expected
        {
            return Err("F responsibility inventory differs from its sources".into());
        }
        let audit: LicenseAudit = records::read(&records::verify(root, &baseline.license_audit)?)?;
        records::version(audit.layout_version)?;
        records::verify(root, &audit.upstream_license)?;
        let patch: ForkPatch = records::read(&records::verify(root, &baseline.patch)?)?;
        records::version(patch.layout_version)?;
        scope(&patch.files)?;
        if patch.revision != baseline.revision
            || patch.policy != POLICY
            || patch.repository_url != FORK_URL
            || patch.pull_request != "https://github.com/MaaXYZ/MaaFramework/pull/1587"
            || patch.ancestor != BASE
            || patch.head != baseline.commit
            || patch.head_tree != baseline.tree
            || !same(&patch.parent_ancestor, parent)
            || baseline.patch.path != format!("patches/pr-1587/{}/manifest.json", patch.revision)
            || patch.files
                != text(range(&upstream, &patch.head, "files")?)?
                    .lines()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            || patch.commits
                != text(range(&upstream, &patch.head, "commits")?)?
                    .lines()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            || patch.commits.is_empty()
            || patch.head_sources.len() != patch.files.len()
            || patch
                .head_sources
                .iter()
                .map(|source| source.original_path.clone())
                .collect::<BTreeSet<_>>()
                != patch.files.iter().cloned().collect::<BTreeSet<_>>()
            || fs::read(records::verify(root, &patch.patch)?)?
                != range(&upstream, &patch.head, "patch")?
        {
            return Err("F revision patch does not match its complete cumulative Git range".into());
        }
        for source in &patch.head_sources {
            verify_source(root, &upstream, &patch.head, source)?;
        }
    }
    if index.patches.iter().any(|patch| {
        patch.path != "patches/pr-1587/manifest.json" && !patch_paths.contains(&patch.path)
    }) {
        return Err("orphan or unknown fork revision patch".into());
    }
    if index.selected_baseline.is_some() {
        selected(root, index)?;
    }
    Ok(())
}
pub(super) fn review_context(root: &Path, parent: &Artifact) -> Result<ReviewContext> {
    let path = records::verify(root, parent)?;
    let baseline: ForkBaseline = records::read(&path)?;
    let upstream = path.parent().unwrap().join("reference/upstream");
    let mut required_sources: BTreeSet<_> = baseline
        .sources
        .into_iter()
        .map(|source| source.original_path)
        .collect();
    required_sources.extend(
        files_at(&upstream, &baseline.commit)?
            .into_iter()
            .filter(|path| {
                path.starts_with("include/MaaControlUnit/")
                    || path.starts_with("source/MaaToolkit/Portal/")
                    || path.starts_with("include/MaaToolkit/Portal/")
                    || [
                        "include/MaaFramework/MaaDef.h",
                        "include/MaaFramework/MaaPort.h",
                        "include/MaaFramework/Instance/MaaCustomController.h",
                    ]
                    .contains(&path.as_str())
            }),
    );
    Ok(ReviewContext {
        upstream,
        commit: baseline.commit,
        required_sources,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_stale_heads_branches_and_unrelated_fork_changes() {
        let head = "1".repeat(40);
        identity("client-exact-v1", &head).unwrap();
        for head in [
            BASE,
            HEAD,
            "main",
            "0000000000000000000000000000000000000000",
        ] {
            assert!(identity("client-exact-v1", head).is_err());
        }
        for revision in ["", "../revision", "Uppercase", "with/slash"] {
            assert!(identity(revision, &head).is_err());
        }
        scope(&FORK_FILES.map(str::to_owned)).unwrap();
        for files in [
            vec![],
            vec![PATCH_FILE.into(), PATCH_FILE.into()],
            vec![
                PATCH_FILE.into(),
                "source/MaaFramework/unrelated.cpp".into(),
            ],
        ] {
            assert!(scope(&files).is_err());
        }
    }
    #[test]
    fn reads_historical_indexes_without_selecting_them_as_corrected_forks() {
        let index: Index =
            serde_json::from_str(r#"{"layout_version":1,"baselines":[],"patches":[],"runs":[]}"#)
                .unwrap();
        assert!(index.selected_baseline.is_none());
        assert!(selected(Path::new("."), &index).is_err());
        assert!(
            !serde_json::to_string(&index)
                .unwrap()
                .contains("selected_baseline")
        );
    }
}
