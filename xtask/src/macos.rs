// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use crate::{
    Result, batch,
    records::{self, Index},
    repository,
};
use std::{
    env, fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn run(args: &[String]) -> Result<()> {
    if !args.is_empty() || !cfg!(target_os = "macos") || env::consts::ARCH != "aarch64" {
        return Err("macos-prototype requires a native macOS arm64 host and no arguments".into());
    }
    let path = env::var_os("MFS_REFERENCE_ROOT")
        .map(Into::into)
        .unwrap_or_else(|| {
            repository()
                .parent()
                .unwrap()
                .join("maafw-screencap-reference")
        });
    let root = records::external_root(&path)?;
    crate::reference::verify(&root)?;
    let before: Index = records::read(&root.join("manifest.json"))?;
    let rust_result = batch::run(&[]);
    let index: Index = records::read(&root.join("manifest.json"))?;
    if index.runs.len() <= before.runs.len() {
        return Err("Rust resolution did not produce a record".into());
    }
    let parent = index.runs.last().unwrap().clone();
    let rust_run: batch::Run = records::read(&records::verify(&root, &parent)?)?;
    let directory = root.join("runs").join(format!(
        "p0-macos-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::create_dir(&directory)?;
    let inputs = directory.join("inputs");
    fs::create_dir(&inputs)?;
    let mut snapshot = batch::Run {
        layout_version: records::LAYOUT_VERSION,
        run_id: directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        phase: "P0".into(),
        phase_status: "incomplete".into(),
        checks: Vec::new(),
        inputs: Vec::new(),
        pending: Vec::new(),
    };
    batch::snapshot_source(&root, &inputs, &mut snapshot)?;
    let source = inputs.join("product-source");
    let lock = rust_run
        .inputs
        .iter()
        .find(|record| record.path.ends_with("/inputs/Cargo.lock"))
        .ok_or("Rust resolution lockfile missing")?;
    let destination = source.join("Cargo.lock");
    fs::copy(records::verify(&root, lock)?, &destination)?;
    snapshot
        .inputs
        .push(records::artifact(&root, &destination)?);
    let deps = repository().parent().unwrap().join("maafw-screencap-deps");
    let status = Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(source.join("scripts/prepare-macos.ps1"))
        .arg("-ReferenceRoot")
        .arg(&root)
        .arg("-DepsRoot")
        .arg(&deps)
        .arg("-RunRoot")
        .arg(&directory)
        .arg("-ConfigureTool")
        .arg(env::current_exe()?)
        .arg("-RustBatch")
        .arg(&parent.path)
        .current_dir(&source)
        .env("CARGO_HOME", deps.join("cargo-home"))
        .env("CARGO_TARGET_DIR", deps.join("target"))
        .status()?;
    let result = directory.join("native-result.json");
    let mut run: batch::Run = records::read(&result)?;
    run.inputs.extend(snapshot.inputs);
    run.inputs.push(parent);
    run.inputs.push(records::artifact(&root, &result)?);
    let manifest = directory.join("manifest.json");
    records::create_json(&manifest, &run)?;
    batch::verify(&root, &records::artifact(&root, &manifest)?)?;
    batch::register(&root, &manifest)?;
    println!(
        "ScreenCaptureKit prototype evidence archived at {}",
        manifest.display()
    );
    rust_result?;
    if !status.success() || run.checks.iter().any(|check| check.status != "passed") {
        return Err("ScreenCaptureKit prototype did not pass every gate; inspect build/capture/missing-environment evidence".into());
    }
    Ok(())
}
