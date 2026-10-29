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
    if !args.is_empty() || !cfg!(target_os = "macos") || env::consts::ARCH != "aarch64" {
        return Err(
            "macOS prototype/source context requires a native macOS arm64 host and no arguments"
                .into(),
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
            "p1-macos-context"
        } else {
            "p0-macos"
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
        let scope = inputs.join("macos-context-scope.md");
        fs::write(
            &scope,
            "# P1 partial Rust macOS migration and original ScreenCaptureKit context\n\nCompile the complete fixed ScreenCaptureKitScreencap header and Objective-C++ source unchanged, including image_from_sample_buffer, with explicit original manual reference counting. Extract its screenshot base declaration without body rewrites; private namespace/logging overlays exclude unrelated framework/input code. Latest formal Xcode and OpenCV core/imgproc retain native arm64 NEON/CPU/HAL optimization; Intel IPP is inapplicable. No OpenCV enters the capture crate or default product dependency tree.\n\nThe self-owned AppKit fixture creates/accesses/closes windows on the process main thread. Joined caller threads invoke the original synchronous method while that main thread pumps fixture events. Every original content query, first matching window or first enumerated display choice, configuration, callback conversion and release, and five-second dispatch semaphore wait remain untouched. Fixture presentation delay precedes capture. An eight-second fixture watchdog ends only an isolated failed process; it does not detach a caller or cancel native work.\n\nA warmup plus twenty actual window/source lifecycles require sixty BGR owners, strict 43x27 known interior pixels within each 47x31 image, joined callers, missing-window nullopt and CPU images retained through source/window destruction and a separate reader. Complete saved rows include unasserted edges. Native FD/thread/Mach-port counts are saved as observations only, never called balance assertions. A separate first-display process requires an engaged nonempty BGR image and retained owner, but no known-color full-display pixel assertion. Permissions missing at preflight return missing_environment without prompting or modifying TCC.\n\nThe initial private macos-screen-capture-kit feature now uses maintained objc2/block2/dispatch2 bindings. Exact native semaphore create/wait/signal and the five-second deadline point remain source behavior; the MRC source create +1 is deliberately retained, with separate test-only reclamation after all caller/callback owners end. This is not product cleanup success. Frozen create+copied-block filter/config references release at their original points; no native handle gains a general Send/Sync implementation. An extra checked packed BGRA copy occurs while locked; actual C++/OpenCV conversion still executes in that callback before Unlock. Missing image/base remains engaged empty output; lock/format/layout rejection, fallible CPU owners and contained Rust/C++ exceptions are explicit safety adaptations.\n\nIndependent owned main-thread scene processes provide one window ID and an unchanged-source oracle for the SAME window. Exact owning-module Rust processes test dimensions/layout, twenty lifecycles/sixty held raw+BGR images, conversion-before-Unlock, native input/request owners, actual conversion exception/recovery and missing window, five-second native timeout with six-second fixture-only conversion delay and late work/owner/module retention, and first-display layout without known pixels. Scene close acknowledgement precedes cross-thread CPU Mat release. Rust FD counts are compared only AFTER explicit fixture reclamation; thread/Mach-port numbers remain observations. Private C headers/native layout queries and actual adapter consumption verify pointer/record widths; only the independent C++ CPU Mat owner is Send in acceptance, not arbitrary native objects.\n\nThe private macos-manager slice reuses this same capture implementation, with Input=None and no SDK policy. Extract constructor/connect/connected/request_uuid/screencap/inactive bodies verbatim from fixed MacOSControlUnitMgr.cpp and check their field declarations against the fixed header. A 320-case original/Rust oracle uses scripted availability/backend constructor/capture outcomes and real C++ Mat outputs; its private C header, complete record layout and actual function consumption are checked. These are not native OS-version rejection or allocator/OOM tests. Separate original/Rust twenty-manager/sixty-image probes use the same owned window, preserve no capture during connect, output on failure and CPU image owners after manager/window shutdown. Manager-only direct build/dependency isolation is distinct from capture-only isolation. All newly added manager probes await native validation; metadata/full API/host/failure matrices remain pending.\n\nThis remains an initial migration, not full backend/P1 acceptance. Source semaphore/port-growth causality and exact residual parity, original timeout/exception/failure controls, full format/empty/lock/geometry/permission/display pixels, complete manager forwarding/API/metadata and all other backend gates remain pending. No P2 patch or P3 SDK behavior is implemented.\n",
        )?;
        snapshot.inputs.push(records::artifact(&root, &scope)?);
        snapshot.inputs.push(index.baselines[0].clone());
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
        .arg(&parent.path);
    if reference {
        let baseline = records::verify(&root, &index.baselines[0])?;
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
    println!(
        "ScreenCaptureKit native evidence archived at {}",
        manifest.display()
    );
    rust_result?;
    if !status.success() || run.checks.iter().any(|check| check.status != "passed") {
        return Err("ScreenCaptureKit native checks did not pass every gate; inspect build/capture/missing-environment evidence".into());
    }
    Ok(())
}
