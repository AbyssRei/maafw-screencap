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
    run_native(args, None)
}

pub fn run_parity(args: &[String]) -> Result<()> {
    let kind = match args.first().map(String::as_str) {
        Some(kind @ ("gdi" | "print-window" | "screen-dc")) => kind,
        _ => return Err("implemented parity entry: upstream-parity <gdi|print-window|screen-dc> [--offline-inputs <native run manifest>]".into()),
    };
    let mut native_args = vec!["--desktop".to_owned()];
    native_args.extend_from_slice(&args[1..]);
    run_native(&native_args, Some(kind))
}

fn run_native(args: &[String], parity: Option<&str>) -> Result<()> {
    let gdi_parity = parity.is_some();
    let print_parity = parity == Some("print-window");
    let screen_parity = parity == Some("screen-dc");
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
        return Err("C++/OpenCV native preparation currently supports Windows x64 only".into());
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
    let source_index: Index = records::read(&root.join("manifest.json"))?;
    let source_baseline = crate::reference::selected(&root, &source_index)?;
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
    let baseline = records::verify(&root, &source_baseline)?;
    let upstream = baseline.parent().unwrap().join("reference/upstream");
    let directory = root.join("runs").join(format!(
        "{}-{}",
        if print_parity {
            "p1-print-window-windows"
        } else if screen_parity {
            "p1-screen-dc-windows"
        } else if gdi_parity {
            "p1-gdi-windows"
        } else {
            "p0-native-windows"
        },
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
        phase: if gdi_parity { "P1" } else { "P0" }.into(),
        phase_status: "incomplete".into(),
        checks: Vec::new(),
        pending: Vec::new(),
        inputs: Vec::new(),
    };
    batch::snapshot_source(&root, &inputs, &mut source_snapshot)?;
    if screen_parity {
        let scope = inputs.join("screen-dc-scope.md");
        fs::write(
            &scope,
            "# P1 ScreenDC migration component scope\n\n\
             Fixed unpatched ScreenDCScreencap and ForegroundUtils/HwndUtils; U retains rcWork, source sleep requests, global DWORD cooldown and original asynchronous window adjustment. Foreground/geometry warning results do not abort capture. A null target is invalid; GetDC/ReleaseDC use the null screen owner and BitBlt uses the client screen origin. Conversion precedes native cleanup, with the same shared GetBitmapBits safety checks as window GDI. No P2 adjustment or P3 SDK/DIB behavior is introduced.\n\n\
             Compiled exact-original headers are observed synchronously: 11 continuous cooldown steps and two direct topmost cases; 16 geometry, three window-size, four style and four DPI cases. Overflow safety cases never execute undefined C++ arithmetic. Scripted native capture tests separately check warning continuation, client-left/top input, negative source origin, native/short-read/conversion failures, independent cleanup errors and unwind cleanup. These scripted boundaries are not real native failure acceptance.\n\n\
             Desktop test requests 20 paired original/Rust captures with exact known pixels, retained CPU outputs and real GDI counts after capture/window close. Fixture-only controls distinguish cold CreateWindow process initialization from capture accounting. Source-only before/after submission/presentation observations retain both images. A later formal run failed both-source/Rust and post-barrier source black-image assertions; those raw images and failures remain archived. The fixture now reuses a dedicated WM_PAINT/UI owner with joined shutdown, without compiling the PrintWindow backend in a ScreenDC-only target. Strict pixels and capture calls/waits remain unchanged, without retry. Outcomes must verify this changed fixture premise; a DwmFlush result alone does not prove readiness.\n\n\
             Occlusion, real native failure/cleanup matrices, asynchronous move/resize, nondefault depth, helper compatible-mode path, complete method/manager chains and full P1 acceptance remain outstanding. Native outcomes are platform/run specific.\n",
        )?;
        source_snapshot
            .inputs
            .push(records::artifact(&root, &scope)?);
    }
    if gdi_parity {
        let scope = inputs.join("gdi-scope.md");
        fs::write(
            &scope,
            "# P1 GDI migration component scope\n\n\
             Pinned GdiScreencap.{h,cpp} and HwndUtils.hpp::window_size/conversion order; the selected F parent identity is retained in this run.\n\
             U-WIN-GDI-01 partial: controlled asymmetric BGR pixels, odd widths, conversion before GDI cleanup, retained CPU output and twenty cycles. Nondefault display depth remains unexecuted.\n\
             U-WIN-GDI-02 native scenarios: null/destroyed/empty/iconic HWND, DC/bitmap/select/copy/read/convert failures, each cleanup failure and a compound primary-plus-cleanup failure. Fault-instrumented original and the same Rust capture share real GDI boundary responses; compare order/results, report residuals before separate fixture recovery and verify CPU owner releases.\n\
             Safe defect observations: the instrumented reference initializes input for positive short-return/layout probes; Rust rejects incomplete reads and failed metadata queries. A real 24-bit DDB may be rejected at SelectObject on the current 32-bit device; this is not a nondefault-depth display capture.\n\
             Safety adaptations: checked arithmetic/narrowing and fallible initialized allocation; GetObjectW validates the original packed one-plane 32-bit DDB assumption; short GetBitmapBits reads and invalid SelectObject sentinels are rejected. These are not DIB modernization.\n\
             No manager/selection/forwarding or other backend case is closed. Actual native outcomes remain in the check logs; no complete GDI backend, F snapshot, P1 exit, public SDK, PR or P3 feature is asserted by this component run.\n",
        )?;
        source_snapshot
            .inputs
            .push(records::artifact(&root, &scope)?);
    }
    if print_parity {
        let scope = inputs.join("print-window-scope.md");
        fs::write(
            &scope,
            "# P1 PrintWindow migration component scope\n\n\
             Pinned PrintWindowScreencap.{h,cpp}; the GDI regression scope is retained separately.\n\
             U-WIN-PRINT-01/02 partial: owned WM_PAINT/WM_PRINTCLIENT class and real PrintWindow(3), main/popup BGR conversion before cleanup, composition after both cleanups, negative/partial/outside/hidden/absent and zero-size popup scenes, twenty fixture lifecycles each, retained CPU ownership. Actual rendering message counts remain observable; no single message path is assumed. Four hundred compiled OpenCV/Rust rectangle boundaries preserve representable empty/clipped results.\n\
             A real child target inside a larger root also runs six scenes for twenty fixture lifecycles each: no root popup, visible/negative/outside root-owned popup, hidden and zero-width popup. Exact child pixels/dimensions, root != target, conversion/composition counts, cleanup and retained CPU output distinguish the child from its root. Hidden fixture setup verifies the resulting native visibility. The fixture independently owns its callback module; an explicit native close after Adapter destruction verifies code/context retention through class unregistration. Forgetting the guard retains that lease rather than unloading live callback code.\n\
             Twenty-nine ordinary native fault scenarios compare isolated exact original function bodies with Rust: main/popup client rectangle, DC/bitmap/selection/PrintWindow/read/conversion failures, each cleanup failure and compound primary/cleanup failures. The ledger retains both HWND resource sets, records residuals before separate fixture recovery and compares attempts/errors/CPU owners without treating that recovery as successful product cleanup. Six additional read-deselect/reselect/short-scanline scenarios classify safety adaptations; the instrumented short-read source input is fully defined. Successful GetLastError values remain raw observations, not guaranteed error contracts.\n\
             U-WIN-PRINT-03 partial: null/destroyed main and zero-size optional popup. Main/popup conversion exceptions in isolated original function bodies and the Rust converter boundary abort the whole operation. Real GDI counts before conversion/after unwind and exported image-owner counts remain distinct from the original internal Mat lifetime: a per-output allocator observes its main BGR allocation/release bytes, without changing the default allocator or observing input BGRA/other OpenCV allocations. Fault modes export no Mat referencing that observer; unexpected outstanding allocations fail and retain the module for late release. Eight query scenarios compare isolated original function bodies and Rust through real native query boundaries: normal/null root, null/root-equal popup, invalid/hidden popup and failure of either ClientToScreen query, including short-circuit order and main-image fallback. Full foreground/background/pseudo-minimize/window-state coverage remains pending.\n\
             The private PseudoMinimizeHelper and PrintWindow wrapper migration preserves synchronous ensure, 100ms polling, partial apply failures, foreground/target-loss observation, inactive restoration and join-before-stop restoration/re-minimization. Twenty-two exact original-body scripts use explicit fake scheduling and compare native calls, sleep requests, failure codes and state snapshots; they do not prove OS thread ownership or desktop pixels. Separate real-worker synchronization checks join order. Native fixtures now own distinct UI threads and real message loops, reading statistics and performing window/class shutdown on those owners, then joining before context/module release. Initial setup draining is bounded; partial construction and Adapter-first destruction check worker/module ownership. Layered/nonlayered stop/inactive and target-loss lifecycles each retain strict known pixels and CPU owners after shutdown. Earlier same-thread layered failures remain archived; independent pumped-owner controls correct the fixture without changing capture/helper calls, waits or conversion timing. Foreground recovery is a separate exact native case; verified missing foreground control is missing_environment, never a passing case, and other independent checks continue. No input injection or activation bypass is added. Earlier popup failures remain unresolved despite later non-reproduction. No helper/wrapper or PrintWindow exit is asserted.\n\
             Safety adaptation: temporarily deselect bitmap only around GetDIBits to meet its documented precondition, reselect before original conversion point, retain final restore/delete/delete/release. A real native selected-object witness verifies both boundaries. DWORD-aligned initialized u32 storage, checked CPU spans/scanline counts and signed geometry reject unsafe source assumptions; no CreateDIBSection/P3 migration is introduced.\n\
             An earlier formal native run returned a black original popup bitmap while the subsequent Rust capture rendered the fixture pattern. Same-bitmap selected/unselected/repeated/raw reads confirmed black contents; later query/flush controls and an exact earlier-fixture replay did not reproduce it and do not establish a fix. Preserve that unresolved parity failure independently of this run's check outcomes; no PrintWindow component exit is asserted.\n\
             No full backend, F snapshot, P1 exit, public SDK, manager, PR or later product feature is accepted by this record.\n",
        )?;
        source_snapshot
            .inputs
            .push(records::artifact(&root, &scope)?);
    }
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
    if gdi_parity {
        process.arg("-GdiParity");
    }
    if print_parity {
        process.arg("-PrintWindowParity");
    }
    if screen_parity {
        process.arg("-ScreenDcParity");
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
    run.inputs.push(source_baseline);
    run.inputs.push(records::artifact(&root, &result)?);
    run.inputs.extend(source_snapshot.inputs);
    let manifest = directory.join("manifest.json");
    records::create_json(&manifest, &run)?;
    let artifact = records::artifact(&root, &manifest)?;
    batch::verify(&root, &artifact)?;
    batch::register(&root, &manifest)?;
    println!(
        "Native {} evidence archived at {}",
        if print_parity {
            "P1 PrintWindow component"
        } else if screen_parity {
            "P1 ScreenDC component"
        } else if gdi_parity {
            "P1 GDI component"
        } else {
            "P0"
        },
        manifest.display()
    );
    if !status.success() || run.checks.iter().any(|check| check.status != "passed") {
        return Err("native preparation failed; the exact attempt and logs were preserved".into());
    }
    Ok(())
}
