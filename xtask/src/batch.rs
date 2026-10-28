// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use crate::{
    Result, command, git,
    records::{self, Artifact, Index, LAYOUT_VERSION},
    repository,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub(crate) layout_version: u32,
    pub(crate) run_id: String,
    pub(crate) phase: String,
    pub(crate) phase_status: String,
    pub(crate) checks: Vec<Check>,
    pub(crate) pending: Vec<String>,
    pub(crate) inputs: Vec<Artifact>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Check {
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) log: Artifact,
}

pub fn run(args: &[String]) -> Result<()> {
    run_at(args, None)
}

pub(crate) fn prepare_at(root: &Path) -> Result<()> {
    run_at(&["--prepare-only".into()], Some(root.to_owned()))
}

fn run_at(args: &[String], root_override: Option<PathBuf>) -> Result<()> {
    let (desktop, prepare_only) = match args {
        [] => (false, false),
        [flag] if flag == "--desktop" => (true, false),
        [flag] if flag == "--prepare-only" => (false, true),
        _ => return Err("expected batch [--desktop | --prepare-only]".into()),
    };
    if desktop && !cfg!(target_os = "windows") {
        return Err("native desktop prototypes for this host are not implemented yet".into());
    }
    let path = root_override
        .or_else(|| env::var_os("MFS_REFERENCE_ROOT").map(Into::into))
        .unwrap_or_else(|| {
            repository()
                .parent()
                .unwrap()
                .join("maafw-screencap-reference")
        });
    let root = records::external_root(&path)?;
    let id = format!(
        "p0-{}-{}",
        env::consts::OS,
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let directory = root.join("runs").join(&id);
    fs::create_dir_all(directory.parent().unwrap())?;
    fs::create_dir(&directory)?;
    let inputs = directory.join("inputs");
    let logs = directory
        .join(format!("{}-{}-nightly", env::consts::OS, env::consts::ARCH))
        .join("logs");
    fs::create_dir(&inputs)?;
    fs::create_dir_all(&logs)?;
    let deps = repository().parent().unwrap().join("maafw-screencap-deps");
    let build_env = (deps.join("cargo-home"), deps.join("target"));
    fs::create_dir_all(&build_env.0)?;
    fs::create_dir_all(&build_env.1)?;
    let mut run = Run {
        layout_version: LAYOUT_VERSION,
        run_id: id,
        phase: "P0".into(),
        phase_status: "incomplete".into(),
        checks: Vec::new(),
        inputs: Vec::new(),
        pending: vec![
            "P0 exit assessment must associate the external source/method catalog, scoped licence review, minimal unpatched reference/adapter and native prototype records; this Rust batch alone does not accept P0".into(),
            "Verify current native toolchains and local plus three-platform Actions prototype/configuration results in their actual environments".into(),
            "Full migrated-backend U parity belongs to P1; full SDK failure/format/recovery and distribution matrices remain in their assigned later phases, not implicit extra P0 gates".into(),
        ],
    };
    // Keep the input source before any command. Edits while dependency queries
    // or native checks run must never be attributed to one coherent batch.
    let source_before = snapshot_source(&root, &inputs, &mut run)?;
    let head_before = fs::read(inputs.join("product-git-head.txt"))?;
    // Update the toolchain before overriding CARGO_HOME in child builds.
    check(
        &root,
        &logs,
        &mut run,
        "nightly-update",
        "rustup",
        &["update", "nightly", "--no-self-update"],
        None,
    )?;
    if passed(&run, "nightly-update") {
        check(
            &root,
            &logs,
            &mut run,
            "dependency-update",
            "cargo",
            &["update"],
            Some(&build_env),
        )?;
    }
    for (name, program) in [("toolchain.txt", "rustc"), ("cargo.txt", "cargo")] {
        let path = inputs.join(name);
        fs::write(&path, command(program, &["-vV".as_ref()], repository())?)?;
        run.inputs.push(records::artifact(&root, &path)?);
    }
    if passed(&run, "dependency-update") {
        // Resolve the graph that can actually be built on this native runner.
        // An unfiltered graph includes, for example, Windows-only dependencies
        // of Linux-only helpers and invents shared nonbinding upgrade gaps.
        // All features remain enabled; each platform has its own native batch.
        let identity = fs::read_to_string(inputs.join("toolchain.txt"))?;
        let target = identity
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .filter(|target| !target.is_empty())
            .ok_or("rustc host missing; cannot select the native dependency graph")?;
        let target_path = inputs.join("dependency-target.txt");
        fs::write(&target_path, format!("{target}\n"))?;
        run.inputs.push(records::artifact(&root, &target_path)?);
        let metadata = cargo_output(
            &[
                "metadata",
                "--format-version=1",
                "--all-features",
                "--filter-platform",
                target,
            ],
            &build_env,
        )?;
        let metadata_path = inputs.join("cargo-metadata.json");
        fs::write(&metadata_path, &metadata)?;
        run.inputs.push(records::artifact(&root, &metadata_path)?);
        let lock = inputs.join("Cargo.lock");
        fs::copy(repository().join("Cargo.lock"), &lock)?;
        run.inputs.push(records::artifact(&root, &lock)?);
        let latest_path = inputs.join("latest-crates.json");
        let (status, message) = match crate::dependencies::latest(&metadata) {
            Ok(value) => {
                let all_latest = value
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|entry| entry["satisfies_policy"] == true);
                records::create_json(&latest_path, &value)?;
                run.inputs.push(records::artifact(&root, &latest_path)?);
                if all_latest {
                    (
                        "passed",
                        "Latest direct dependencies and required transitive releases verified; binding constraint gaps are recorded in latest-crates.json.".into(),
                    )
                } else {
                    ("failed", "Resolved packages lag latest stable releases; inspect latest-crates.json and adapt.".into())
                }
            }
            Err(error) => ("failed", error.to_string()),
        };
        let log = logs.join("latest-major-check.log");
        fs::write(&log, message)?;
        run.checks.push(Check {
            name: "latest-major-check".into(),
            status: status.into(),
            log: records::artifact(&root, &log)?,
        });
        let vendor = inputs.join("vendor");
        check(
            &root,
            &logs,
            &mut run,
            "offline-inputs",
            "cargo",
            &[
                "vendor",
                "--locked",
                "--respect-source-config",
                "--versioned-dirs",
                vendor.to_str().ok_or("non-UTF-8 vendor path")?,
            ],
            Some(&build_env),
        )?;
        if vendor.exists() {
            archive_files(&root, &vendor, &mut run.inputs)?;
        }
        if !prepare_only {
            for (name, args) in [
                ("release-build", vec!["build", "--release", "--locked"]),
                ("workspace-tests", vec!["test", "--workspace", "--locked"]),
                ("format", vec!["fmt", "--all", "--", "--check"]),
                (
                    "clippy",
                    vec![
                        "clippy",
                        "--workspace",
                        "--all-targets",
                        "--locked",
                        "--",
                        "-D",
                        "warnings",
                    ],
                ),
            ] {
                check(
                    &root,
                    &logs,
                    &mut run,
                    name,
                    "cargo",
                    &args,
                    Some(&build_env),
                )?;
            }
            // Clippy can change Cargo's fingerprint for xtask. Re-enter the running
            // executable without asking Cargo to replace that live file on Windows.
            let executable = env::current_exe()?;
            check(
                &root,
                &logs,
                &mut run,
                "project-conventions",
                executable.to_str().ok_or("non-UTF-8 xtask executable")?,
                &["check"],
                Some(&build_env),
            )?;
            if desktop {
                check(
                    &root,
                    &logs,
                    &mut run,
                    "windows-native-prototype",
                    "cargo",
                    &[
                        "test",
                        "--release",
                        "--locked",
                        "--test",
                        "p0-interop",
                        "--features",
                        "prototype-windows-com",
                        "--",
                        "--include-ignored",
                        "--test-threads=1",
                        "--nocapture",
                    ],
                    Some(&build_env),
                )?;
                check(
                    &root,
                    &logs,
                    &mut run,
                    "windows-prototype-clippy",
                    "cargo",
                    &[
                        "clippy",
                        "--workspace",
                        "--all-targets",
                        "--locked",
                        "--features",
                        "prototype-windows-com",
                        "--",
                        "-D",
                        "warnings",
                    ],
                    Some(&build_env),
                )?;
            } else {
                run.pending
                    .push("Native desktop capture not requested in this batch".into());
            }
        } else {
            run.pending.push("Dependency preparation only: no product build, test, desktop capture or standalone consumer was executed.".into());
        }
    }
    if !prepare_only && passed(&run, "offline-inputs") {
        offline_smoke(&root, &deps, &inputs, &logs, &mut run)?;
    }
    let source_after = source_fingerprint(repository());
    let head_after = git(repository(), &["rev-parse", "HEAD"]);
    let stable = source_after
        .as_ref()
        .is_ok_and(|after| after == &source_before)
        && head_after.as_ref().is_ok_and(|after| after == &head_before);
    let mut message =
        String::from("Checks compare source bytes and Git HEAD at the batch boundaries.\n");
    match source_after {
        Ok(after) => {
            for name in source_before
                .keys()
                .chain(after.keys())
                .collect::<std::collections::BTreeSet<_>>()
            {
                if source_before.get(name) != after.get(name) {
                    message.push_str(&format!("changed/added/removed source: {name}\n"));
                }
            }
        }
        Err(error) => message.push_str(&format!("Cannot fingerprint final source: {error}\n")),
    }
    match head_after {
        Ok(after) if after != head_before => {
            message.push_str("Git HEAD differs from the archived starting revision.\n")
        }
        Err(error) => message.push_str(&format!("Cannot read final Git HEAD: {error}\n")),
        _ => {}
    }
    if stable {
        message.push_str("Source and Git HEAD matched the archived starting input.\n");
    } else {
        message.push_str("Source/HEAD changed or could not be verified; this batch cannot pass.\n");
        let changed = inputs.join("changed-source");
        fs::create_dir(&changed)?;
        if let Err(error) = snapshot_source(&root, &changed, &mut run) {
            message.push_str(&format!("Final source snapshot is incomplete: {error}\n"));
        }
    }
    let log = logs.join("source-stability.log");
    fs::write(&log, message)?;
    run.checks.push(Check {
        name: "source-stability".into(),
        status: if stable { "passed" } else { "failed" }.into(),
        log: records::artifact(&root, &log)?,
    });
    let failed = run.checks.iter().any(|check| check.status != "passed");
    let manifest = directory.join("manifest.json");
    records::create_json(&manifest, &run)?;
    register(&root, &manifest)?;
    println!(
        "Archived run {}. P0 remains incomplete; see manifest pending items.",
        manifest.display()
    );
    if failed {
        return Err("checks failed; logs are preserved without retry".into());
    }
    Ok(())
}

fn check(
    root: &Path,
    logs: &Path,
    run: &mut Run,
    name: &str,
    program: &str,
    args: &[&str],
    build_env: Option<&(PathBuf, PathBuf)>,
) -> Result<()> {
    println!("Running {name} ...");
    let mut process = Command::new(program);
    process.args(args).current_dir(repository());
    if let Some((home, target)) = build_env {
        process
            .env("CARGO_HOME", home)
            .env("CARGO_TARGET_DIR", target);
    }
    let (status, output) = match process.output() {
        Ok(output) => (
            if output.status.success() {
                "passed"
            } else {
                "failed"
            },
            format!(
                "{program} {args:?}\nexit: {}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        ),
        Err(error) => ("failed", format!("could not start {program}: {error}")),
    };
    let path = logs.join(format!("{name}.log"));
    fs::write(&path, output)?;
    run.checks.push(Check {
        name: name.into(),
        status: status.into(),
        log: records::artifact(root, &path)?,
    });
    println!("{name}: {status}");
    Ok(())
}

fn cargo_output(args: &[&str], build_env: &(PathBuf, PathBuf)) -> Result<Vec<u8>> {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(repository())
        .env("CARGO_HOME", &build_env.0)
        .env("CARGO_TARGET_DIR", &build_env.1)
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string().into());
    }
    Ok(output.stdout)
}

fn passed(run: &Run, name: &str) -> bool {
    run.checks
        .iter()
        .any(|check| check.name == name && check.status == "passed")
}

fn source_names(directory: &Path) -> Result<Vec<String>> {
    let names = git(
        directory,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    names
        .split(|b| *b == 0)
        .filter(|name| !name.is_empty())
        .map(|name| Ok(std::str::from_utf8(name)?.to_owned()))
        .collect()
}

pub(crate) fn source_fingerprint(directory: &Path) -> Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    for name in source_names(directory)? {
        let path = directory.join(&name);
        if path.is_file() {
            files.insert(name, records::hash(&path)?);
        }
    }
    Ok(files)
}

pub(crate) fn snapshot_source(
    root: &Path,
    inputs: &Path,
    run: &mut Run,
) -> Result<BTreeMap<String, String>> {
    let source = inputs.join("product-source");
    fs::create_dir(&source)?;
    let mut files = BTreeMap::new();
    for name in source_names(repository())? {
        let from = repository().join(&name);
        if !from.is_file() {
            continue;
        }
        let to = source.join(&name);
        fs::create_dir_all(to.parent().unwrap())?;
        fs::copy(from, &to)?;
        let artifact = records::artifact(root, &to)?;
        files.insert(name, artifact.sha256.clone());
        run.inputs.push(artifact);
    }
    for required in [
        "Cargo.toml",
        "build.rs",
        "build-support/windows-prototype.txt",
        "build-support/windows-com-prototype.txt",
        "scripts/archive-evidence.ps1",
        "scripts/prepare-pipewire.ps1",
        "scripts/prepare-macos.ps1",
        "scripts/prepare-wlr.ps1",
        "tests/fixtures/pipewire-probe.conf",
        "tests/fixtures/pipewire-source.c",
        "tests/fixtures/wlr-compositor.c",
        "tests/prototypes/macos_main.rs",
        "tests/prototypes/macos_screen_capture.rs",
        "xtask/src/main.rs",
    ] {
        if !source.join(required).is_file() {
            return Err(format!(
                "required source input omitted from snapshot (check Git ignore rules): {required}"
            )
            .into());
        }
    }
    for (name, args) in [
        ("product-git-head.txt", vec!["rev-parse", "HEAD"]),
        ("working-tree.patch", vec!["diff", "HEAD", "--binary"]),
    ] {
        let path = inputs.join(name);
        fs::write(&path, git(repository(), &args)?)?;
        run.inputs.push(records::artifact(root, &path)?);
    }
    Ok(files)
}

fn offline_smoke(
    root: &Path,
    deps: &Path,
    inputs: &Path,
    logs: &Path,
    run: &mut Run,
) -> Result<()> {
    // Copy inputs out of reference: the consumer has no sibling reference tree,
    // an empty Cargo home, and only archived sources for offline dependency use.
    let smoke = deps.join("offline-consumers").join(&run.run_id);
    fs::create_dir_all(smoke.parent().unwrap())?;
    fs::create_dir(&smoke)?;
    let source = smoke.join("source");
    copy_tree(&inputs.join("product-source"), &source)?;
    fs::copy(inputs.join("Cargo.lock"), source.join("Cargo.lock"))?;
    let vendor = smoke.join("vendor");
    copy_tree(&inputs.join("vendor"), &vendor)?;
    let configuration = smoke.join("vendor.toml");
    fs::write(
        &configuration,
        format!(
            "[source.crates-io]\nreplace-with = \"mfs-archived\"\n[source.mfs-archived]\ndirectory = {}\n",
            serde_json::to_string(&vendor.to_string_lossy())?
        ),
    )?;
    let manifest = source.join("Cargo.toml");
    let configuration = configuration
        .to_str()
        .ok_or("non-UTF-8 configuration path")?;
    let manifest = manifest.to_str().ok_or("non-UTF-8 consumer manifest")?;
    let build_env = (smoke.join("empty-cargo-home"), smoke.join("target"));
    check(
        root,
        logs,
        run,
        "offline-standalone-default",
        "cargo",
        &[
            "build",
            "--release",
            "--offline",
            "--locked",
            "--no-default-features",
            "--manifest-path",
            manifest,
            "--config",
            configuration,
        ],
        Some(&build_env),
    )?;
    let mut args = vec![
        "test",
        "--release",
        "--offline",
        "--locked",
        "--manifest-path",
        manifest,
        "--config",
        configuration,
        "--test",
        "p0-interop",
        "--no-run",
    ];
    if cfg!(target_os = "windows") {
        args.extend(["--features", "prototype-windows-com"]);
    }
    check(
        root,
        logs,
        run,
        "offline-standalone-prototype",
        "cargo",
        &args,
        Some(&build_env),
    )?;
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &path)?;
        } else if entry.file_type()?.is_file() {
            fs::copy(entry.path(), path)?;
        } else {
            return Err("unexpected symlink in standalone consumer inputs".into());
        }
    }
    Ok(())
}

fn archive_files(root: &Path, directory: &Path, artifacts: &mut Vec<Artifact>) -> Result<()> {
    let mut entries = fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if entry.file_type()?.is_dir() {
            archive_files(root, &entry.path(), artifacts)?;
        } else if entry.file_type()?.is_file() {
            artifacts.push(records::artifact(root, &entry.path())?);
        } else {
            return Err("unexpected symlink in archived crate inputs".into());
        }
    }
    Ok(())
}

pub(crate) fn register(root: &Path, manifest: &Path) -> Result<()> {
    use std::io::Write;
    let path = root.join("manifest.json");
    let mut index: Index = if path.exists() {
        records::read(&path)?
    } else {
        Index {
            layout_version: LAYOUT_VERSION,
            ..Index::default()
        }
    };
    records::version(index.layout_version)?;
    index.runs.push(records::artifact(root, manifest)?);
    let temporary = root.join(format!("manifest-next-{}.json", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(&index)?)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)?;
    Ok(())
}

pub fn verify(root: &Path, artifact: &Artifact) -> Result<()> {
    let run: Run = records::read(&records::verify(root, artifact)?)?;
    records::version(run.layout_version)?;
    let component_scope = if run.run_id.starts_with("p1-gdi-windows-") {
        Some("gdi-scope.md")
    } else if run.run_id.starts_with("p1-print-window-windows-") {
        Some("print-window-scope.md")
    } else if run.run_id.starts_with("p1-screen-dc-windows-") {
        Some("screen-dc-scope.md")
    } else if run.run_id.starts_with("p1-frame-pool-context-windows-") {
        Some("frame-pool-context-scope.md")
    } else if run.run_id.starts_with("p1-frame-pool-windows-") {
        Some("frame-pool-scope.md")
    } else if run.run_id.starts_with("p1-desktop-dup-windows-") {
        Some("desktop-dup-scope.md")
    } else {
        None
    };
    let supported_phase = run.phase == "P0"
        || (run.phase == "P1"
            && component_scope.is_some()
            && run
                .inputs
                .iter()
                .filter(|input| {
                    (input.path.starts_with("baselines/u/")
                        || input.path.starts_with("baselines/f/"))
                        && input.path.ends_with("/manifest.json")
                })
                .count()
                == 1
            && run.inputs.iter().any(|input| {
                input.path == format!("runs/{}/inputs/{}", run.run_id, component_scope.unwrap())
            }));
    if !supported_phase || run.phase_status != "incomplete" || run.checks.is_empty() {
        return Err("unsupported preparation/component run record".into());
    }
    for artifact in run
        .inputs
        .iter()
        .chain(run.checks.iter().map(|check| &check.log))
    {
        records::verify(root, artifact)?;
    }
    for check in &run.checks {
        if !["passed", "failed", "missing_environment"].contains(&check.status.as_str()) {
            return Err("invalid check status".into());
        }
    }
    for input in run
        .inputs
        .iter()
        .filter(|input| input.path.ends_with("/inputs/llvm-toolchain.json"))
    {
        crate::native_tools::verify(root, input)?;
    }
    if run.run_id.starts_with("p0-catalog-") {
        let path = format!("runs/{}/inputs/capture-catalog.json", run.run_id);
        let catalog = run
            .inputs
            .iter()
            .find(|input| input.path == path)
            .ok_or("catalog run does not retain its validated definition")?;
        crate::review::verify(root, catalog)?;
    }
    if run.run_id.starts_with("p0-licenses-") {
        let path = format!("runs/{}/inputs/registry-licenses.json", run.run_id);
        let inventory = run
            .inputs
            .iter()
            .find(|input| input.path == path)
            .ok_or("license run does not retain its registry inventory")?;
        crate::licenses::verify(root, inventory)?;
    }
    if run.run_id.starts_with("p0-config-") {
        let path = format!("runs/{}/inputs/configuration.json", run.run_id);
        let configuration = run
            .inputs
            .iter()
            .find(|input| input.path == path)
            .ok_or("configuration run does not retain its native discovery record")?;
        crate::configuration::verify(root, configuration)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SourceFixture(PathBuf);
    impl Drop for SourceFixture {
        fn drop(&mut self) {
            let base = env::temp_dir().canonicalize().unwrap();
            let target = self.0.canonicalize().unwrap();
            assert!(target.starts_with(&base) && target != base);
            fs::remove_dir_all(target).unwrap();
        }
    }

    #[test]
    fn component_evidence_requires_scope_and_parent_and_cannot_accept_a_phase() {
        let root = env::temp_dir().join(format!(
            "mfs-component-record-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let fixture = SourceFixture(root);
        for kind in [
            "gdi",
            "print-window",
            "screen-dc",
            "frame-pool-context",
            "frame-pool",
            "desktop-dup",
        ] {
            let run_id = format!("p1-{kind}-windows-fixture");
            let scope = fixture
                .0
                .join(format!("runs/{run_id}/inputs/{kind}-scope.md"));
            let baseline = fixture.0.join("baselines/u/source/manifest.json");
            fs::create_dir_all(scope.parent().unwrap()).unwrap();
            fs::create_dir_all(baseline.parent().unwrap()).unwrap();
            fs::write(&scope, "Partial component scope, not a phase acceptance.\n").unwrap();
            fs::write(
                &baseline,
                "Fixture parent identity; not a real U baseline.\n",
            )
            .unwrap();
            let log = fixture.0.join("component.log");
            fs::write(&log, "Fixture component check.\n").unwrap();
            let mut run = Run {
                layout_version: records::LAYOUT_VERSION,
                run_id,
                phase: "P1".into(),
                phase_status: "incomplete".into(),
                checks: vec![Check {
                    name: "fixture".into(),
                    status: "passed".into(),
                    log: records::artifact(&fixture.0, &log).unwrap(),
                }],
                pending: vec!["Full U remains unexecuted.".into()],
                inputs: vec![
                    records::artifact(&fixture.0, &scope).unwrap(),
                    records::artifact(&fixture.0, &baseline).unwrap(),
                ],
            };
            for (index, accepted) in [true, false, false, false, true, false]
                .into_iter()
                .enumerate()
            {
                match index {
                    1 => run.phase_status = "complete".into(),
                    2 => {
                        run.phase_status = "incomplete".into();
                        run.inputs.pop();
                    }
                    3 => {
                        run.inputs
                            .push(records::artifact(&fixture.0, &baseline).unwrap());
                        run.inputs.remove(0);
                    }
                    4 => {
                        let fork = fixture.0.join("baselines/f/source/manifest.json");
                        fs::create_dir_all(fork.parent().unwrap()).unwrap();
                        fs::write(&fork, "Fixture F parent identity, not native acceptance.\n")
                            .unwrap();
                        run.inputs = vec![
                            records::artifact(&fixture.0, &scope).unwrap(),
                            records::artifact(&fixture.0, &fork).unwrap(),
                        ];
                    }
                    5 => run
                        .inputs
                        .push(records::artifact(&fixture.0, &baseline).unwrap()),
                    _ => {}
                }
                let manifest = fixture.0.join(format!("{kind}-component-{index}.json"));
                records::create_json(&manifest, &run).unwrap();
                assert_eq!(
                    verify(
                        &fixture.0,
                        &records::artifact(&fixture.0, &manifest).unwrap()
                    )
                    .is_ok(),
                    accepted
                );
            }
        }
    }

    #[test]
    fn detects_changed_added_and_removed_source_but_excludes_ignored_outputs() {
        let path = env::temp_dir().join(format!(
            "mfs-source-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        let fixture = SourceFixture(path);
        git(&fixture.0, &["init", "--quiet"]).unwrap();
        fs::write(fixture.0.join(".gitignore"), "output\n").unwrap();
        fs::write(fixture.0.join("tracked.rs"), "original").unwrap();
        git(&fixture.0, &["add", ".gitignore", "tracked.rs"]).unwrap();
        let before = source_fingerprint(&fixture.0).unwrap();
        fs::write(fixture.0.join("output"), "generated bytes").unwrap();
        assert_eq!(before, source_fingerprint(&fixture.0).unwrap());
        fs::write(fixture.0.join("tracked.rs"), "edited").unwrap();
        fs::write(fixture.0.join("new.rs"), "new untracked source").unwrap();
        let after = source_fingerprint(&fixture.0).unwrap();
        assert_ne!(before["tracked.rs"], after["tracked.rs"]);
        assert!(after.contains_key("new.rs"));
        fs::remove_file(fixture.0.join("tracked.rs")).unwrap();
        assert!(
            !source_fingerprint(&fixture.0)
                .unwrap()
                .contains_key("tracked.rs")
        );
    }
}
