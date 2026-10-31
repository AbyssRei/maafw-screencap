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
    run_native(args, false)
}

pub fn run_reference(args: &[String]) -> Result<()> {
    run_native(args, true)
}

fn run_native(args: &[String], reference: bool) -> Result<()> {
    if !args.is_empty() || !cfg!(target_os = "linux") || env::consts::ARCH != "x86_64" {
        return Err(
            "Wlr prototype/source context requires a native Linux x64 host and no arguments".into(),
        );
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
    let source_baseline = reference
        .then(|| crate::reference::selected(&root, &before))
        .transpose()?;
    let rust_result = batch::run(&[]);
    let index: Index = records::read(&root.join("manifest.json"))?;
    if index.runs.len() <= before.runs.len() {
        return Err("Rust resolution did not produce a record".into());
    }
    let parent = index.runs.last().unwrap().clone();
    let rust_run: batch::Run = records::read(&records::verify(&root, &parent)?)?;
    let directory = root.join("runs").join(format!(
        "{}-{}",
        if reference {
            "p1-wlr-context-linux"
        } else {
            "p0-wlr-linux"
        },
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
        phase: if reference { "P1" } else { "P0" }.into(),
        phase_status: "incomplete".into(),
        checks: Vec::new(),
        inputs: Vec::new(),
        pending: Vec::new(),
    };
    batch::snapshot_source(&root, &inputs, &mut snapshot)?;
    if reference {
        let scope = inputs.join("wlr-context-scope.md");
        fs::write(
            &scope,
            "# P1 partial Wlr migration and original context\n\nCompile complete fixed WlrScreencap, WaylandClient, MemfdBuffer, WaylandHelper and generated C protocols unchanged. Use the exact extracted screenshot base, private namespace/logging overlays and an explicit kernel-random UUID name provider instead of unrelated Boost/MaaUtils UUID code. This is not a comparison of that UUID utility. Native Wayland/SHM/Mat/OpenCV capture paths remain real.\n\nOwned headless/Pixman scene only. Its explicit source-client mode supplies passive virtual keyboard/pointer manager globals required by the original client; no input actions or host devices. Twenty connection/capture/inactive/close lifecycles retain sixty known BGR images through native destruction and require exact FD counts and task identities. A forwarding linker wrapper observes recursive wl_display_roundtrip, preserving each real call and return. Capture waits/pixels are not retried. Build, library identity, native capture and compositor shutdown are distinct recorded checks.\n\nPrivate linux-wlr uses maintained bindings/ObjectData and a one-function C queue getter absent from wayland-sys. Recursive native roundtrip stays inside active dispatcher TLS, with no mutable-state lock or borrow across it. Frame survives actual C++/OpenCV conversion. A separately counted raw CPU copy preserves the source Mat packed stride and ignored inversion flags; no P3 correction or performance claim. Twenty Rust lifecycles require sixty held raw/BGR images, exact resources, SHM overwrites and conversion/frame lifetime witnesses. Separate processes test checked layouts and actual conversion exception/release/subsequent capture. Ordinary buffer-preparation failure leaves waiting unchanged; scalar evidence does not add early wake or timeout. Our callback panic boundary does not recover arbitrary binding/native aborting OOM.\n\nSeparate unchanged-source contexts now probe two simultaneous captures retaining one client after caller release, first-source destruction before another capture, and final client release over twenty cycles/sixty held images. Another twenty-cycle context checks same-path no-op, close/reopen, changed-path alias, missing-path failure and subsequent connect only after all old captures/proxy owners end. An isolated original MemfdBuffer FD-zero probe observes its retained descriptor/mapping, then explicitly reclaims both and restores fixture stdin; that is not product cleanup success. These source contexts pass the scoped native batch and independent image verification. Rust now separates shared client registry/connection ownership from per-capture callback/buffer/result state and retains the same Arc client plus copied manager/SHM/output snapshots. Two new isolated twenty-cycle native tests require sixty shared-client and twenty safe-reconnect raw/BGR/source pairs, caller/first/final-owner release, independent buffers, callback accounting and exact FD/task baselines. The previously validated shared/reconnect slice required unique client access after old snapshots ended. The current provisional safety adaptation instead lets idle handles retain an exact Arc channel through proxy/buffer destruction: logical close clears globals immediately, native disconnect waits for those owners, and a stale source rejects a new current channel before marshalling. Active control/capture/conversion rejects close/connect reentry without holding a state lock over dispatch. One native-channel Arc and a temporary retained-path copy on actual open are added; frame-copy and conversion timing stay unchanged. A new 80-case native matrix and independent 382-file checker must validate the adaptation; old pass counts do not validate this new lifetime behavior. The changed Rust shared/reconnect component now passes all 93 native checks; sixty shared and twenty reconnected raw/BGR/source pairs plus forty resource rows independently verify. After the original probe demonstrated the residual, Buffer now explicitly retains successful FD-zero mapping/file owners, while ordinary destruction remains proxy/pool then unmap/close. The twenty-cycle Rust native probe checks capture, residuals, idle-map overwrite/CPU isolation and separate fixture reclaim/stdin restoration; it has passed the 91-check native component and independent image verification. Failed-construction/partial-state FD-zero parity remains pending.\n\nChecked sizes/narrowing, missing-binding-version and RAII owners are explicit safety adaptations. Native names now retain the original filesystem::path::c_str boundary: embedded NUL ends the C name while the full path remains reconnect identity; empty/leading-NUL names use a nonnull pointer. New isolated source/Rust native controls require twenty cycles, sixty retained images each, unchanged-full-name no-op, changed-NUL-suffix reconnect, no default-source selection and a JSON-created Rust manager with complete UUID bytes. These NUL controls pass the 98-check native component; all sixty raw/Rust BGR/source triples and forty resource rows independently verify. A new allocation-free source path comparator retains dot/dot-dot and trailing empty filename components instead of Rust Path normalization. Its 841 actual Linux C++ filesystem comparisons and twenty real dot-reconnect/repeated-separator-no-op/trailing-separator-failure/explicit-recovery/capture lifecycles await native execution. The new retained-channel lifetime adaptation still requires native compilation/execution. Complete error/FD0/partial-state parity, formats/padding/inversion, foreign queues and native mobility remain unfinished. No public SDK, P2 source freeze, P3 worker/target policy or full P1 acceptance.\n",
        )?;
        snapshot.inputs.push(records::artifact(&root, &scope)?);
        snapshot
            .inputs
            .push(source_baseline.as_ref().unwrap().clone());
    }
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
    let mut process = Command::new("pwsh");
    process
        .args(["-NoProfile", "-File"])
        .arg(source.join("scripts/prepare-wlr.ps1"))
        .arg("-ReferenceRoot")
        .arg(&root)
        .arg("-DepsRoot")
        .arg(&deps)
        .arg("-RunRoot")
        .arg(&directory)
        .arg("-ConfigureTool")
        .arg(env::current_exe()?)
        .arg("-RustBatch")
        .arg(&parent.path);
    if reference {
        let baseline = records::verify(&root, source_baseline.as_ref().unwrap())?;
        process
            .arg("-SourceReference")
            .arg("-UpstreamRoot")
            .arg(baseline.parent().unwrap().join("reference/upstream"));
    }
    let status = process
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
    println!("Wlr native evidence archived at {}", manifest.display());
    // A failed dependency/convention gate is retained even when native probes
    // run successfully to collect useful diagnostic evidence. Never waive it.
    rust_result?;
    if !status.success() || run.checks.iter().any(|check| check.status != "passed") {
        return Err("Wlr native checks failed; all attempted results were retained".into());
    }
    Ok(())
}
