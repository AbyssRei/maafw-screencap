// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use crate::{
    Result, command, git,
    records::{self, Artifact, Index, LAYOUT_VERSION},
    repository,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    layout_version: u32,
    run_id: String,
    phase: String,
    phase_status: String,
    checks: Vec<Check>,
    pending: Vec<String>,
    inputs: Vec<Artifact>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Check {
    name: String,
    status: String,
    log: Artifact,
}

pub fn run(args: &[String]) -> Result<()> {
    let desktop = match args {
        [] => false,
        [flag] if flag == "--desktop" => true,
        _ => return Err("expected batch [--desktop]".into()),
    };
    if desktop && !cfg!(target_os = "windows") {
        return Err("native desktop prototypes for this host are not implemented yet".into());
    }
    let path = env::var_os("MFS_REFERENCE_ROOT")
        .map(PathBuf::from)
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
            "Complete responsibility review and method/target parity matrix".into(),
            "Independent unpatched reference executable and latest-source OpenCV adapter".into(),
            "COM/WinRT/D3D, native callback shutdown and FFI prototypes".into(),
            "Linux PipeWire/Wlr and macOS ScreenCaptureKit native prototypes".into(),
            "Three native Actions runner results, platform dependency isolation and AUTO/ON/OFF"
                .into(),
        ],
    };
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
        let metadata = cargo_output(
            &["metadata", "--format-version=1", "--all-features"],
            &build_env,
        )?;
        let metadata_path = inputs.join("cargo-metadata.json");
        fs::write(&metadata_path, &metadata)?;
        run.inputs.push(records::artifact(&root, &metadata_path)?);
        let lock = inputs.join("Cargo.lock");
        fs::copy(repository().join("Cargo.lock"), &lock)?;
        run.inputs.push(records::artifact(&root, &lock)?);
        let latest_path = inputs.join("latest-crates.json");
        let (status, message) = match latest_crates(&metadata) {
            Ok(value) => {
                let all_latest = value
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|entry| entry["resolved"] == entry["latest_stable"]);
                records::create_json(&latest_path, &value)?;
                run.inputs.push(records::artifact(&root, &latest_path)?);
                if all_latest {
                    (
                        "passed",
                        "All registry packages match crates.io latest stable releases.".into(),
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
            ("project-conventions", vec!["xtask", "check"]),
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
                    "prototype-windows",
                    "--",
                    "--include-ignored",
                    "--test-threads=1",
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
                    "prototype-windows",
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
    }
    snapshot_source(&root, &inputs, &mut run)?;
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

fn latest_crates(metadata: &[u8]) -> Result<Value> {
    let metadata: Value = serde_json::from_slice(metadata)?;
    let mut records = Vec::new();
    for package in metadata["packages"].as_array().ok_or("packages missing")? {
        if !package["source"]
            .as_str()
            .is_some_and(|source| source.starts_with("registry+"))
        {
            continue;
        }
        let name = package["name"].as_str().ok_or("package name missing")?;
        let url = format!("https://crates.io/api/v1/crates/{name}");
        let response = command(
            "curl",
            &[
                "--fail".as_ref(),
                "--silent".as_ref(),
                "--show-error".as_ref(),
                "--location".as_ref(),
                "--proto".as_ref(),
                "=https".as_ref(),
                "--user-agent".as_ref(),
                "maafw-screencap-p0".as_ref(),
                url.as_ref(),
            ],
            repository(),
        )?;
        let response: Value = serde_json::from_slice(&response)?;
        let latest = response["crate"]["max_stable_version"]
            .as_str()
            .ok_or("latest stable version missing")?;
        records.push(json!({ "name": name, "resolved": package["version"], "latest_stable": latest, "source": url, "license": package["license"] }));
    }
    Ok(Value::Array(records))
}

fn snapshot_source(root: &Path, inputs: &Path, run: &mut Run) -> Result<()> {
    let source = inputs.join("product-source");
    fs::create_dir(&source)?;
    let names = git(
        repository(),
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    for name in names.split(|b| *b == 0).filter(|name| !name.is_empty()) {
        let name = std::str::from_utf8(name)?;
        let from = repository().join(name);
        if !from.is_file() {
            continue;
        }
        let to = source.join(name);
        fs::create_dir_all(to.parent().unwrap())?;
        fs::copy(from, &to)?;
        run.inputs.push(records::artifact(root, &to)?);
    }
    for required in [
        "Cargo.toml",
        "build.rs",
        "build-support/windows-prototype.txt",
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

fn register(root: &Path, manifest: &Path) -> Result<()> {
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
    if run.phase != "P0" || run.phase_status != "incomplete" || run.checks.is_empty() {
        return Err("unsupported P0 run record".into());
    }
    for artifact in run
        .inputs
        .iter()
        .chain(run.checks.iter().map(|check| &check.log))
    {
        records::verify(root, artifact)?;
    }
    for check in &run.checks {
        if !["passed", "failed"].contains(&check.status.as_str()) {
            return Err("invalid check status".into());
        }
    }
    Ok(())
}
