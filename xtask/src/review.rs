// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// Schema and validation only. Source mappings, observations, review prose and
// parity definitions are supplied externally, never built into the product.
use crate::{
    Result, batch, git,
    records::{self, Artifact},
    reference, repository,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Method {
    Gdi,
    PrintWindow,
    ScreenDc,
    FramePool,
    DesktopDup,
    DesktopDupWindow,
    Pipewire,
    Wlr,
    ScreenCaptureKitWindow,
    ScreenCaptureKitFirstDisplay,
    CustomCallback,
}
impl Method {
    const ALL: [Self; 11] = [
        Self::Gdi,
        Self::PrintWindow,
        Self::ScreenDc,
        Self::FramePool,
        Self::DesktopDup,
        Self::DesktopDupWindow,
        Self::Pipewire,
        Self::Wlr,
        Self::ScreenCaptureKitWindow,
        Self::ScreenCaptureKitFirstDisplay,
        Self::CustomCallback,
    ];
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    layout_version: u32,
    #[serde(alias = "parent_u")]
    parent_source: Artifact,
    reports: Vec<Report>,
    sources: Vec<Source>,
    methods: Vec<MethodDefinition>,
    cases: Vec<Case>,
    remaining: Vec<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Report {
    id: String,
    artifact: Artifact,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ReviewState {
    Reviewed,
    Pending,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Responsibility {
    Backend,
    DirectHelper,
    Manager,
    NativeResource,
    PublicContract,
    Build,
    License,
    SharedProtocol,
    HostAuthorization,
    Excluded,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: String,
    git_blob: String,
    sha256: String,
    responsibility: Responsibility,
    state: ReviewState,
    summary: String,
    report_ids: Vec<String>,
    evidence: Vec<Evidence>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    line: usize,
    text: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MethodDefinition {
    method: Method,
    source_paths: Vec<String>,
    targets: Vec<Target>,
    conversion: String,
    ownership: String,
    execution_channel: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Target {
    kind: String,
    behavior: TargetBehavior,
    explanation: String,
    case_ids: Vec<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum TargetBehavior {
    SourceImplemented,
    Rejected,
    Fallback,
    DeclaredOnly,
    NotExposed,
}

// A definition is not an execution result. Neither passed nor skipped is a
// supported value; native parity results will have their own actual format.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Execution {
    NotRun,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    methods: Vec<Method>,
    source_paths: Vec<String>,
    trigger: String,
    expected: String,
    execution: Execution,
}

fn nonempty(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err("catalog contains an empty description or identifier".into());
    }
    Ok(())
}
fn unique<T: Ord>(items: impl IntoIterator<Item = T>) -> Result<BTreeSet<T>> {
    let mut result = BTreeSet::new();
    for item in items {
        if !result.insert(item) {
            return Err("duplicate catalog identifier or reference".into());
        }
    }
    Ok(result)
}

fn validate_graph(catalog: &Catalog, required: &BTreeSet<String>) -> Result<usize> {
    records::version(catalog.layout_version)?;
    let reports = unique(catalog.reports.iter().map(|r| r.id.as_str()))?;
    if reports.is_empty() {
        return Err("catalog has no review reports".into());
    }
    for id in &reports {
        nonempty(id)?;
    }
    let sources: BTreeMap<_, _> = catalog
        .sources
        .iter()
        .map(|s| (s.path.as_str(), s))
        .collect();
    if sources.len() != catalog.sources.len()
        || required.iter().any(|s| !sources.contains_key(s.as_str()))
    {
        return Err(
            "catalog has duplicate sources or omits fixed discovery/header/portal coverage".into(),
        );
    }
    for source in &catalog.sources {
        nonempty(&source.summary)?;
        let report_ids = unique(source.report_ids.iter().map(String::as_str))?;
        if report_ids.iter().any(|id| !reports.contains(id)) {
            return Err("source cites an unknown review report".into());
        }
        if source.state == ReviewState::Reviewed
            && (source.evidence.is_empty() || report_ids.is_empty())
        {
            return Err("reviewed source requires anchored evidence and a report".into());
        }
    }
    let methods = unique(catalog.methods.iter().map(|m| m.method))?;
    if methods != Method::ALL.into_iter().collect() {
        return Err("catalog must cover all eleven fixed upstream capture routes".into());
    }
    let case_ids = unique(catalog.cases.iter().map(|c| c.id.as_str()))?;
    if case_ids.is_empty() {
        return Err("catalog has no parity case definitions".into());
    }
    for case in &catalog.cases {
        if !case.id.starts_with("U-")
            || !case
                .id
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err("parity case requires a U-prefixed portable identifier".into());
        }
        nonempty(&case.trigger)?;
        nonempty(&case.expected)?;
        if unique(case.methods.iter().copied())?.is_empty() {
            return Err("case has no capture route".into());
        }
        reviewed_sources(&case.source_paths, &sources)?;
    }
    for method in &catalog.methods {
        reviewed_sources(&method.source_paths, &sources)?;
        nonempty(&method.conversion)?;
        nonempty(&method.ownership)?;
        nonempty(&method.execution_channel)?;
        if method.targets.is_empty() {
            return Err("capture route has no target matrix".into());
        }
        unique(method.targets.iter().map(|t| t.kind.as_str()))?;
        for target in &method.targets {
            nonempty(&target.kind)?;
            nonempty(&target.explanation)?;
            if unique(target.case_ids.iter().map(String::as_str))?.is_empty() {
                return Err("target has no parity case".into());
            }
            for id in &target.case_ids {
                let case = catalog
                    .cases
                    .iter()
                    .find(|case| &case.id == id)
                    .ok_or("target cites an unknown parity case")?;
                if !case.methods.contains(&method.method) {
                    return Err("target cites a case for another capture route".into());
                }
            }
        }
        if !catalog
            .cases
            .iter()
            .any(|c| c.methods.contains(&method.method))
        {
            return Err("capture route has no parity cases".into());
        }
    }
    // Every case must be reachable through a target, so helper/failure scenarios
    // cannot disappear from the matrix behind an unused definition.
    let linked: BTreeSet<_> = catalog
        .methods
        .iter()
        .flat_map(|m| &m.targets)
        .flat_map(|t| &t.case_ids)
        .map(String::as_str)
        .collect();
    if linked != case_ids {
        return Err("parity definitions include cases missing from the target matrix".into());
    }
    for case in &catalog.cases {
        for route in &case.methods {
            let method = catalog.methods.iter().find(|m| &m.method == route).unwrap();
            if !method
                .targets
                .iter()
                .any(|target| target.case_ids.contains(&case.id))
            {
                return Err("case is missing from one of its declared capture routes".into());
            }
        }
    }
    if catalog.remaining.is_empty() {
        return Err("P0 catalog must retain its outstanding acceptance work".into());
    }
    for note in &catalog.remaining {
        nonempty(note)?;
    }
    Ok(catalog
        .sources
        .iter()
        .filter(|s| s.state == ReviewState::Pending)
        .count())
}

fn reviewed_sources(paths: &[String], sources: &BTreeMap<&str, &Source>) -> Result<()> {
    if unique(paths.iter().map(String::as_str))?.is_empty() {
        return Err("definition has no source evidence".into());
    }
    for path in paths {
        let source = sources
            .get(path.as_str())
            .ok_or("definition cites an unknown source")?;
        if source.state != ReviewState::Reviewed
            || matches!(source.responsibility, Responsibility::Excluded)
        {
            return Err("capture definition cites a pending or excluded source".into());
        }
    }
    Ok(())
}

fn verify_source(upstream: &Path, commit: &str, source: &Source) -> Result<()> {
    let path = records::resolve(upstream, &source.path)?;
    if records::hash(&path)? != source.sha256 {
        return Err(format!("catalog source hash mismatch: {}", source.path).into());
    }
    let object = format!("{commit}:{}", source.path);
    let blob = String::from_utf8(git(upstream, &["rev-parse", &object])?)?;
    let bytes = fs::read(path)?;
    if blob.trim() != source.git_blob || bytes != git(upstream, &["show", &object])? {
        return Err(format!(
            "catalog source differs from fixed Git object: {}",
            source.path
        )
        .into());
    }
    verify_lines(&bytes, &source.evidence)
}

fn verify_lines(bytes: &[u8], evidence: &[Evidence]) -> Result<()> {
    let content = std::str::from_utf8(bytes)?;
    unique(evidence.iter().map(|e| e.line))?;
    let lines: Vec<_> = content.lines().collect();
    for item in evidence {
        nonempty(&item.text)?;
        if item
            .line
            .checked_sub(1)
            .and_then(|index| lines.get(index))
            .copied()
            != Some(item.text.as_str())
        {
            return Err("catalog source line evidence does not match the exact source text".into());
        }
    }
    Ok(())
}

pub(crate) fn verify(root: &Path, artifact: &Artifact) -> Result<usize> {
    let catalog: Catalog = records::read(&records::verify(root, artifact)?)?;
    validate(root, &catalog)
}

fn validate(root: &Path, catalog: &Catalog) -> Result<usize> {
    let context = reference::review_context(root, &catalog.parent_source)?;
    let pending = validate_graph(catalog, &context.required_sources)?;
    for report in &catalog.reports {
        records::verify(root, &report.artifact)?;
    }
    for source in &catalog.sources {
        verify_source(&context.upstream, &context.commit, source)?;
    }
    Ok(pending)
}

fn retain_dependency_snapshot(root: &Path, run: &mut batch::Run) -> Result<()> {
    let lock = fs::read(repository().join("Cargo.lock"))?;
    let identity = crate::command("rustc", &["-vV".as_ref()], repository())?;
    let index: records::Index = records::read(&root.join("manifest.json"))?;
    let prefix = format!("runs/p0-{}-", env::consts::OS);
    for artifact in index
        .runs
        .iter()
        .rev()
        .filter(|r| r.path.starts_with(&prefix))
    {
        let batch: batch::Run = records::read(&records::verify(root, artifact)?)?;
        let Some(saved_lock) = batch
            .inputs
            .iter()
            .find(|i| i.path.ends_with("/inputs/Cargo.lock"))
        else {
            continue;
        };
        let Some(saved_identity) = batch
            .inputs
            .iter()
            .find(|i| i.path.ends_with("/inputs/toolchain.txt"))
        else {
            continue;
        };
        if fs::read(records::verify(root, saved_lock)?)? == lock
            && fs::read(records::verify(root, saved_identity)?)? == identity
            && batch
                .inputs
                .iter()
                .any(|i| i.path.contains("/inputs/vendor/"))
        {
            // Reuse only the matching dependency/toolchain archive, not that
            // batch's test claims for different source. No capture build runs.
            run.inputs
                .extend([artifact.clone(), saved_lock.clone(), saved_identity.clone()]);
            return Ok(());
        }
    }
    run.pending.push("No matching archived Cargo.lock/toolchain/vendor batch found for catalog-tool offline replay; catalog validation is not build acceptance.".into());
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    let (input, root) = match args {
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
            "expected reference catalog <reference-relative input> [--root <external directory>]"
                .into(),
        ),
    };
    let root = records::external_root(&root)?;
    reference::verify(&root)?;
    let submitted = records::resolve(&root, input)?;
    let mut catalog: Catalog = records::read(&submitted)?;
    let pending = validate(&root, &catalog)?;
    let id = format!(
        "p0-catalog-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let directory = root.join("runs").join(&id);
    fs::create_dir(&directory)?;
    let inputs = directory.join("inputs");
    fs::create_dir(&inputs)?;
    let mut run = batch::Run {
        layout_version: records::LAYOUT_VERSION,
        run_id: id,
        phase: "P0".into(),
        phase_status: "incomplete".into(),
        checks: Vec::new(),
        inputs: Vec::new(),
        pending: catalog.remaining.clone(),
    };
    batch::snapshot_source(&root, &inputs, &mut run)?;
    retain_dependency_snapshot(&root, &mut run)?;
    let submitted_copy = inputs.join("submitted-catalog.json");
    fs::copy(submitted, &submitted_copy)?;
    run.inputs.push(records::artifact(&root, &submitted_copy)?);
    for (number, report) in catalog.reports.iter_mut().enumerate() {
        let path = inputs.join(format!("review-{number}.md"));
        fs::copy(records::verify(&root, &report.artifact)?, &path)?;
        report.artifact = records::artifact(&root, &path)?;
        run.inputs.push(report.artifact.clone());
    }
    let archived = inputs.join("capture-catalog.json");
    records::create_json(&archived, &catalog)?;
    let artifact = records::artifact(&root, &archived)?;
    verify(&root, &artifact)?;
    run.inputs.extend([artifact, catalog.parent_source.clone()]);
    let log = directory.join("catalog-validation.log");
    fs::write(
        &log,
        format!(
            "Validated {} sources, {} capture routes and {} unexecuted case definitions; {pending} source reviews pending.\nExact Git objects, SHA-256, line anchors, report hashes and target/case references checked.\nNo native parity execution, license acceptance or phase completion is asserted.\n",
            catalog.sources.len(),
            catalog.methods.len(),
            catalog.cases.len()
        ),
    )?;
    run.checks.push(batch::Check {
        name: "catalog-structure-and-source-evidence".into(),
        status: "passed".into(),
        log: records::artifact(&root, &log)?,
    });
    let manifest = directory.join("manifest.json");
    records::create_json(&manifest, &run)?;
    batch::verify(&root, &records::artifact(&root, &manifest)?)?;
    batch::register(&root, &manifest)?;
    println!(
        "Catalog archived at {}. {pending} source reviews pending; every submitted case remains unexecuted and no phase completion is asserted.",
        manifest.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn fixture() -> Value {
        let methods: Vec<_> = Method::ALL.iter().enumerate().map(|(n, method)| json!({"method":method,"source_paths":["capture.cpp"],"targets":[{"kind":"fixture","behavior":"source_implemented","explanation":"source-defined target","case_ids":[format!("U-CASE-{n}")]}],"conversion":"original point","ownership":"owned output","execution_channel":"native owner"})).collect();
        let cases: Vec<_> = Method::ALL.iter().enumerate().map(|(n, method)| json!({"id":format!("U-CASE-{n}"),"methods":[method],"source_paths":["capture.cpp"],"trigger":"controlled source","expected":"compare native evidence","execution":"not_run"})).collect();
        json!({"layout_version":1,"parent_u":{"path":"baseline.json","sha256":"0".repeat(64)},"reports":[{"id":"review","artifact":{"path":"review.md","sha256":"0".repeat(64)}}],"sources":[{"path":"capture.cpp","git_blob":"1".repeat(40),"sha256":"0".repeat(64),"responsibility":"backend","state":"reviewed","summary":"reviewed source behavior","report_ids":["review"],"evidence":[{"line":1,"text":"capture"}]}],"methods":methods,"cases":cases,"remaining":["Native U parity has not run"]})
    }
    fn graph(value: Value, required: BTreeSet<String>) -> Result<usize> {
        validate_graph(&serde_json::from_value(value)?, &required)
    }
    #[test]
    fn requires_all_capture_routes_and_declared_source_coverage() {
        assert_eq!(
            graph(fixture(), BTreeSet::from(["capture.cpp".into()])).unwrap(),
            0
        );
        assert!(graph(fixture(), BTreeSet::from(["omitted.h".into()])).is_err());
        let mut missing = fixture();
        missing["methods"].as_array_mut().unwrap().pop();
        assert!(graph(missing, BTreeSet::new()).is_err());
    }
    #[test]
    fn rejects_duplicate_and_unreachable_cases() {
        let mut duplicate = fixture();
        let case = duplicate["cases"][0].clone();
        duplicate["cases"].as_array_mut().unwrap().push(case);
        assert!(graph(duplicate, BTreeSet::new()).is_err());
        let mut orphan = fixture();
        orphan["cases"][0]["id"] = "U-ORPHAN".into();
        assert!(graph(orphan, BTreeSet::new()).is_err());
    }
    #[test]
    fn rejects_cases_for_other_routes_and_pending_source_claims() {
        let mut wrong = fixture();
        wrong["methods"][0]["targets"][0]["case_ids"][0] = "U-CASE-1".into();
        assert!(graph(wrong, BTreeSet::new()).is_err());
        let mut pending = fixture();
        pending["sources"][0]["state"] = "pending".into();
        assert!(graph(pending, BTreeSet::new()).is_err());
        let mut omitted = fixture();
        omitted["cases"][0]["methods"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::to_value(Method::FramePool).unwrap());
        assert!(graph(omitted, BTreeSet::new()).is_err());
    }
    #[test]
    fn cannot_serialize_acceptance_as_an_unexecuted_definition() {
        let mut passed = fixture();
        passed["cases"][0]["execution"] = "passed".into();
        assert!(serde_json::from_value::<Catalog>(passed).is_err());
        let mut extra = fixture();
        extra["phase_status"] = "complete".into();
        assert!(serde_json::from_value::<Catalog>(extra).is_err());
    }
    #[test]
    fn requires_exact_nonempty_one_based_source_line_evidence() {
        let check = |line, text: &str| {
            verify_lines(
                b"first\nsecond\n",
                &[Evidence {
                    line,
                    text: text.into(),
                }],
            )
        };
        assert!(check(2, "second").is_ok());
        assert!(check(0, "first").is_err());
        assert!(check(3, "second").is_err());
        assert!(check(2, "first").is_err());
        assert!(check(1, "").is_err());
    }

    #[test]
    fn rejects_source_replacement_even_with_a_matching_submitted_hash() {
        struct Fixture(std::path::PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let base = env::temp_dir().canonicalize().unwrap();
                let path = self.0.canonicalize().unwrap();
                assert!(path.starts_with(&base) && path != base);
                fs::remove_dir_all(path).unwrap();
            }
        }
        let path = env::temp_dir().join(format!(
            "mfs-review-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        let checkout = Fixture(path);
        fs::write(checkout.0.join("capture.cpp"), "capture\n").unwrap();
        git(&checkout.0, &["init", "--quiet"]).unwrap();
        git(&checkout.0, &["add", "capture.cpp"]).unwrap();
        git(
            &checkout.0,
            &[
                "-c",
                "user.name=P0 Tests",
                "-c",
                "user.email=p0-tests@invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        )
        .unwrap();
        let commit = String::from_utf8(git(&checkout.0, &["rev-parse", "HEAD"]).unwrap()).unwrap();
        let mut source: Source = serde_json::from_value(fixture()["sources"][0].clone()).unwrap();
        source.git_blob =
            String::from_utf8(git(&checkout.0, &["rev-parse", "HEAD:capture.cpp"]).unwrap())
                .unwrap()
                .trim()
                .into();
        source.sha256 = records::hash(&checkout.0.join("capture.cpp")).unwrap();
        assert!(verify_source(&checkout.0, commit.trim(), &source).is_ok());
        fs::write(checkout.0.join("capture.cpp"), "replacement\n").unwrap();
        assert!(verify_source(&checkout.0, commit.trim(), &source).is_err());
        source.sha256 = records::hash(&checkout.0.join("capture.cpp")).unwrap();
        assert!(verify_source(&checkout.0, commit.trim(), &source).is_err());
    }
}
