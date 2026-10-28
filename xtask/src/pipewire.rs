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
            "PipeWire prototype/source context requires a native Linux x64 host and no arguments"
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
            "p1-pipewire-context-linux"
        } else {
            "p0-pipewire-linux"
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
        let scope = inputs.join("pipewire-context-scope.md");
        fs::write(
            &scope,
            "# P1 original PipeWire native source context\n\nInclude the fixed complete unedited PipeWireScreencap cpp in one native Linux x64 test translation unit against the selected latest PipeWire/SPA and OpenCV core/imgproc, retaining IPP/CPU optimizations. Extract only the original screenshot base declaration to exclude input/ConfigBase/meojson; explicit private namespace and suppressed logging are test build adaptations, not full framework or timing parity. Native DRM declarations come from the latest formal libdrm source.\n\nUse the owned asymmetric padded-RGB native producer and private daemon, plus an independent exact-source route supervisor supplying the external AUTOCONNECT routing premise. It validates full node info and only the explicit owned producer/consumer pair, never host/default sources, retains non-lingering proxies and requires nonzero created/bound/destroyed equality after core-sync shutdown. Product connection/properties/waits and P0 explicit routing are unchanged. The P0 native Rust bar probe is a separate entry, not run on this pattern. Observe the real connect_fd early-error descriptor state in an isolated process, with cleanup restricted to the same socket cookie after native teardown. Twenty independent fd/connect/init/three-call/inactive/close/destruction cycles check known BGR pixels, source connected state, fd closure and observed process fd/thread baselines. Retain sixty CPU Mat images through source close and validate across another thread. Save target-only source images before pixel assertions. The harness balances source constructor pw_init only after original destruction; original destructor still lacks pw_deinit. The source public API does not expose freshness/replay; elapsed times are diagnostic.\n\nBuild, load/linkage, actual source output, private Rust consumer unit checks and P0 native Rust stream probe are separate evidence. A separate first native Rust daemon/SHM probe compares three exact source negotiation PODs and requires twenty lifecycles/sixty raw RGB outputs, native queue returns, fd/thread baselines and retained CPU samples. RawAudit does not perform OpenCV conversion. A separate explicit private C++/OpenCV adapter and ABI/exception/owner test plus twenty-cycle/sixty-BGR probe require actual cvtColor after native unlock. The private header layout and real consumption are checked; one native callback-progress witness per cycle adds diagnostic waiting, not source latency parity. Opaque Mat/loader mobility and complete native BGR parity remain pending. Checked CPU target-row storage rejects unsupported spans/offsets and allocates a new owner while shared, an explicit safety adaptation to source mutable BGR cache aliasing. Both U constructors retain raw per-instance pw_init without destructor deinit; only the harness balances after complete native teardown. Portal FD, owned mapping/DMA-BUF, complete format/failure/alias matrices, framework/input and all-platform U/P1 remain unfinished or unvalidated. No P2/P3 or public SDK acceptance.\n",
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
        .arg(source.join("scripts/prepare-pipewire.ps1"))
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
    println!(
        "PipeWire native context/prototype evidence archived at {}",
        manifest.display()
    );
    // A failed dependency/convention gate is retained even when native probes
    // run successfully to collect useful diagnostic evidence. Never waive it.
    rust_result?;
    if !status.success() || run.checks.iter().any(|check| check.status != "passed") {
        return Err(
            "PipeWire native context/prototype checks failed; all attempted results were retained"
                .into(),
        );
    }
    Ok(())
}
