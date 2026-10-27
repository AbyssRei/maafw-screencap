// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use crate::{
    Result, batch,
    records::{self, Index},
    repository,
};
use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn run(args: &[String]) -> Result<()> {
    let mut desktop = false;
    let mut offline_input = None;
    let mut args = args.iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--desktop" if !desktop => desktop = true,
            "--offline-inputs" if offline_input.is_none() => {
                offline_input = Some(
                    args.next()
                        .ok_or("--offline-inputs requires a reference-relative run manifest")?,
                );
            }
            _ => {
                return Err("expected native [--desktop] [--offline-inputs <run manifest>]".into());
            }
        }
    }
    if !cfg!(target_os = "windows") || env::consts::ARCH != "x86_64" {
        return Err("C++/OpenCV P0 preparation currently supports Windows x64 only".into());
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
    crate::reference::verify(&root)?;
    // Freeze one updated Rust resolution before building/testing the explicitly
    // selected native acceptance adapter. This remains independent of capture.
    if offline_input.is_none() {
        let batch_args = if desktop {
            vec!["--desktop".into()]
        } else {
            Vec::new()
        };
        batch::run(&batch_args)?;
    }
    let index: Index = records::read(&root.join("manifest.json"))?;
    let offline = offline_input
        .map(|path| {
            index
                .runs
                .iter()
                .find(|record| &record.path == path)
                .cloned()
                .ok_or("offline input run must be registered in this reference index")
        })
        .transpose()?;
    let parent = if let Some(input) = &offline {
        let prepared: batch::Run = records::read(&records::verify(&root, input)?)?;
        prepared
            .inputs
            .iter()
            .find(|record| {
                record.path.starts_with("runs/p0-windows-")
                    && record.path.ends_with("/manifest.json")
            })
            .cloned()
            .ok_or("offline native inputs do not identify a Rust resolution batch")?
    } else {
        index
            .runs
            .last()
            .ok_or("Rust validation batch missing")?
            .clone()
    };
    let parent_run: batch::Run = records::read(&records::verify(&root, &parent)?)?;
    if parent_run
        .checks
        .iter()
        .any(|check| check.status != "passed")
    {
        return Err("the selected Rust dependency batch contains failures".into());
    }
    if offline.is_some() {
        let identity = parent_run
            .inputs
            .iter()
            .find(|record| record.path.ends_with("/inputs/toolchain.txt"))
            .ok_or("offline toolchain identity missing")?;
        let actual = crate::command("rustc", &["-vV".as_ref()], repository())?;
        if fs::read(records::verify(&root, identity)?)? != actual {
            return Err(
                "installed Rust toolchain differs from the explicit offline snapshot".into(),
            );
        }
    }
    let baseline = records::verify(&root, &index.baselines[0])?;
    let upstream = baseline.parent().unwrap().join("reference/upstream");
    let directory = root.join("runs").join(format!(
        "p0-native-windows-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::create_dir(&directory)?;
    let inputs = directory.join("inputs");
    fs::create_dir(&inputs)?;
    let mut source_snapshot = batch::Run {
        layout_version: records::LAYOUT_VERSION,
        run_id: directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        phase: "P0".into(),
        phase_status: "incomplete".into(),
        checks: Vec::new(),
        pending: Vec::new(),
        inputs: Vec::new(),
    };
    batch::snapshot_source(&root, &inputs, &mut source_snapshot)?;
    let lock = parent_run
        .inputs
        .iter()
        .find(|artifact| artifact.path.ends_with("/inputs/Cargo.lock"))
        .ok_or("Rust batch lockfile is missing")?;
    let source = inputs.join("product-source");
    let source_lock = source.join("Cargo.lock");
    fs::copy(records::verify(&root, lock)?, &source_lock)?;
    source_snapshot
        .inputs
        .push(records::artifact(&root, &source_lock)?);
    let deps = repository().parent().unwrap().join("maafw-screencap-deps");
    let mut process = Command::new("pwsh");
    process
        .args(["-NoProfile", "-File"])
        .arg(source.join("scripts/prepare-native.ps1"))
        .arg("-ReferenceRoot")
        .arg(&root)
        .arg("-DepsRoot")
        .arg(&deps)
        .arg("-RunRoot")
        .arg(&directory)
        .arg("-UpstreamRoot")
        .arg(&upstream)
        .arg("-ConfigureTool")
        .arg(env::current_exe()?)
        .arg("-RustBatch")
        .arg(&parent.path)
        .current_dir(&source)
        .env("CARGO_HOME", deps.join("cargo-home"))
        .env("CARGO_TARGET_DIR", deps.join("target"));
    if desktop {
        process.arg("-Desktop");
    }
    if let Some(input) = &offline {
        process
            .arg("-OfflineManifest")
            .arg(records::verify(&root, input)?);
        let vendor = parent_run
            .inputs
            .iter()
            .find(|record| record.path.contains("/inputs/vendor/"))
            .ok_or("offline vendor sources missing")?;
        let (prefix, _) = vendor.path.split_once("/inputs/vendor/").unwrap();
        let directory = records::resolve(&root, &format!("{prefix}/inputs/vendor"))?;
        let configuration = inputs.join("offline-cargo.toml");
        fs::write(
            &configuration,
            format!(
                "[source.crates-io]\nreplace-with = \"mfs-prepared\"\n[source.mfs-prepared]\ndirectory = {}\n",
                serde_json::to_string(&directory.to_string_lossy())?
            ),
        )?;
        source_snapshot
            .inputs
            .push(records::artifact(&root, &configuration)?);
        process
            .arg("-CargoConfig")
            .arg(configuration)
            .env("CARGO_HOME", inputs.join("empty-cargo-home"));
    }
    let status = process.status()?;
    let result = directory.join("native-result.json");
    let mut run: batch::Run = records::read(&result)?;
    run.inputs.push(parent);
    if let Some(input) = offline {
        run.inputs.push(input);
    }
    run.inputs.push(index.baselines[0].clone());
    run.inputs.push(records::artifact(&root, &result)?);
    run.inputs.extend(source_snapshot.inputs);
    let manifest = directory.join("manifest.json");
    records::create_json(&manifest, &run)?;
    let artifact = records::artifact(&root, &manifest)?;
    batch::verify(&root, &artifact)?;
    batch::register(&root, &manifest)?;
    println!("Native P0 evidence archived at {}", manifest.display());
    if !status.success() || run.checks.iter().any(|check| check.status != "passed") {
        return Err("native preparation failed; the exact attempt and logs were preserved".into());
    }
    Ok(())
}
