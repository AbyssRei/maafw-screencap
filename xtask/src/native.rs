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
        Some(kind @ ("gdi" | "print-window" | "screen-dc" | "frame-pool" | "desktop-dup" | "manager")) => kind,
        _ => return Err("implemented parity entry: upstream-parity <gdi|print-window|screen-dc|frame-pool|desktop-dup|manager> [--offline-inputs <native run manifest>]".into()),
    };
    let mut native_args = vec!["--desktop".to_owned()];
    native_args.extend_from_slice(&args[1..]);
    run_native(&native_args, Some(kind))
}

pub fn run_context(args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) == Some("pipewire") {
        return crate::pipewire::run_reference(&args[1..]);
    }
    if args.first().map(String::as_str) == Some("wlr") {
        return crate::wlr::run_reference(&args[1..]);
    }
    if args.first().map(String::as_str) == Some("macos") {
        return crate::macos::run_reference(&args[1..]);
    }
    if args.first().map(String::as_str) != Some("frame-pool") {
        return Err(
            "expected upstream-context frame-pool [--offline-inputs <native run manifest>], upstream-context <pipewire|wlr> (native Linux x64), or upstream-context macos (native macOS arm64)".into(),
        );
    }
    let mut native_args = vec!["--desktop".to_owned()];
    native_args.extend_from_slice(&args[1..]);
    run_native(&native_args, Some("frame-pool-context"))
}

pub fn run_model(args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) != Some("pipewire") {
        return Err(
            "expected upstream-model pipewire [--offline-inputs <native run manifest>]".into(),
        );
    }
    let mut native_args = vec!["--desktop".to_owned()];
    native_args.extend_from_slice(&args[1..]);
    run_native(&native_args, Some("pipewire-control"))
}

fn run_native(args: &[String], parity: Option<&str>) -> Result<()> {
    let frame_context = parity == Some("frame-pool-context");
    let frame_parity = parity == Some("frame-pool");
    let desktop_parity = parity == Some("desktop-dup");
    let manager_parity = parity == Some("manager");
    let pipewire_control = parity == Some("pipewire-control");
    let gdi_parity = parity.is_some() && !frame_context;
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
        "{}-{}",
        if print_parity {
            "p1-print-window-windows"
        } else if screen_parity {
            "p1-screen-dc-windows"
        } else if frame_context {
            "p1-frame-pool-context-windows"
        } else if frame_parity {
            "p1-frame-pool-windows"
        } else if desktop_parity {
            "p1-desktop-dup-windows"
        } else if manager_parity {
            "p1-manager-windows"
        } else if pipewire_control {
            "p1-pipewire-control-windows"
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
        phase: if parity.is_some() { "P1" } else { "P0" }.into(),
        phase_status: "incomplete".into(),
        checks: Vec::new(),
        pending: Vec::new(),
        inputs: Vec::new(),
    };
    batch::snapshot_source(&root, &inputs, &mut source_snapshot)?;
    if pipewire_control {
        let scope = inputs.join("pipewire-control-scope.md");
        fs::write(
            &scope,
            "# P1 partial PipeWire consumer control model\n\nPrivate consumer control flow, not native Linux capture. Compile exact fixed-source loop_wait_until/LoopLocker, format table, connected/inactive/wait_for_frame_locked/screencap bodies with newline normalization only. Explicit fake native loop/clock and symbolic format IDs; real OpenCV Mat ownership and conversion. Compare 74 cases/118 operations: whole-second wait truncation and spurious wakeups, ignored native activation errors, fresh versus three timeout replays/fourth rejection, empty cache, source disconnected predicate, inactive/reactivation, format selection, conversion after unlock, contained conversion exceptions and BGR shared-owner move. Additional checked clock/counter arithmetic remains a safety adaptation outside source undefined-overflow parity.\n\nThis does not compile the full original Linux class, prove SPA numeric values or native lock/callback/stream/buffer/teardown. A future native driver must exclude callbacks while locked and release/reacquire around timed_wait without leaving Rust references into callback-mutated state. Native setup/POD/process/SHM/DMA-BUF, caller mobility, initialization ownership, source manager and complete U remain pending; existing Linux runtime evidence remains P0 only. No P1/Linux-backend acceptance or P2/P3 implementation. Dependency preparation and Windows GDI regressions in this host batch are separate checks.\n",
        )?;
        source_snapshot
            .inputs
            .push(records::artifact(&root, &scope)?);
    }
    if manager_parity {
        let scope = inputs.join("manager-scope.md");
        fs::write(
            &scope,
            "# P1 partial Windows screenshot-manager migration\n\nPrivate screenshot path from fixed Win32ControlUnitMgr, input excluded. Preserve capture reset before caller PMv2 DPI change, subsequent HWND validation, None versus unknown-only selection, construction of all selected candidates before warmup/testing, millisecond-truncated costs and strict-less-than tie order, warmup failure followed by a measured call, selected owner survival, unselected destruction, inactive and output preservation. Some(empty Mat) remains an engaged successful source result. Optional failures, escaping exceptions and additional safety rejections remain distinct, with backend cleanup evidence. U intentionally leaves caller DPI changed; P3 host restoration is not introduced.\n\nNative source bodies and original enum are extracted with newline normalization into the native build, along with the selected MaaUtils duration_since body. A private declaration shim and input=None stubs exclude input code. Real MSVC unordered_map/shared_ptr and OpenCV Mat allocator/deallocation are observed while backend, DPI/window and clock boundaries are explicitly scripted. 122 cases compare ordered construction/capture/temporary-image release/timing/native-container destruction, all 64 recognized flag subsets, failed warmups/tests/construction, exceptions, empty images, output replacement/preservation, stale HWND checks and reconnect. This establishes the current MSVC container behavior, not a portable unordered_map iteration guarantee. The extraction does not compile the complete original input/control manager class.\n\nA separate source-body GDI-only boundary uses the actual original GDI class, real caller DPI/native calls and input=None stubs, with entry probes; other methods are deliberately unavailable in that reference. Twenty paired source/Rust manager/window lifecycles validate two connect calls plus one published capture each, strict odd-width known BGR pixels, CPU/image owners after source/Rust/window/module close, GDI resources and caller PMv2 context. The harness restores its own caller context outside the source/Rust manager. Single-method probes do not validate native competing-method timing.\n\nPending: native six-method/mixed-wrapper selection and winner cleanup/failure matrices, compatibility-mode DPI, complete framework/control-unit forwarding, factory allocation/cleanup faults, all other backend matrices and full all-platform U/P1 acceptance. Previous FramePool/PrintWindow failures and DesktopDup fixture failures remain failed records. No public SDK, P2 or P3 acceptance.\n",
        )?;
        source_snapshot
            .inputs
            .push(records::artifact(&root, &scope)?);
    }
    if desktop_parity {
        let scope = inputs.join("desktop-dup-scope.md");
        fs::write(
            &scope,
            "# P1 partial Desktop Duplication migration\n\nOriginal default hardware D3D11CreateDevice flags zero, CreateDXGIFactory, adapter/output enumeration and primary fallback (adapter 0/output 0, not assumed to be the OS primary monitor), DuplicateOutput, 2000ms acquisitions, up to three discarded last-pixel-alpha warmups followed by a distinct final acquisition, access-lost partial invalidation and native release orders. Window mode retains foreground/rcWork helpers, full-monitor BGR conversion before geometry queries and client ROI.\n\nSafety adaptations: the original returns a cv::Mat borrowing mapped staging storage after Unmap/ReleaseFrame; Rust copies initialized packed BGRA rows while mapped, then converts only after native guards release. This additional full-output CPU copy is counted. Window output additionally clones only target BGR pixels after the original full conversion/crop point; its extra copy and changed allocation/retention are explicit. Null outputs, layout/format/descriptor incompatibility and arithmetic are checked. Native scope contains no callbacks or workers and is caller-bound.\n\nAn isolated executable runs the unmodified original full/window and null-HWND classes once each. The null-HWND preflight independently queries adapter 0/output 0 and requires that attached output to contain the fixture; unmet premises are missing_environment, without selecting a different output. A separate owned marker presents alternating colors outside the static target so original warmup/final Acquire calls can receive updates. Rust tests twenty mixed full/window lifecycles and a separate null-HWND case, source target bytes, exact target pattern, ReleaseFrame HRESULTs, packed target storage and CPU images after native/window shutdown. Full desktop content is not compared byte-for-byte because the marker and external desktop change. Source pixel agreement does not prove its dangling mapped view is valid.\n\nA separate native diagnostic prelude obtains independent staging/interface owners, retaining capture's warmup flag. At normal and throwing external BGR conversion boundaries, native ReleaseFrame must return DXGI_ERROR_INVALID_CALL; staging Map must succeed and input must have independent CPU storage matching its first row. The diagnostic mapping ends before C++ conversion. A contained cv::Exception must preserve successful capture-frame release, exact image-owner counts and retained prior pixels. Map success is not exact Unmap-call interception; this diagnostic is not an original-source call-count comparison.\n\nPending: native faults/release failures/access loss/device/format changes, monitor-lookup fallback/multiadapter/output/rotation scenarios, helper/window moves/occlusion/DPI, deterministic native warmup call/alpha scripts, original repeated lifecycles, exact Unmap interception and manager/selection chain. This is not full backend, U/P1, public SDK or P3 factory/DuplicateOutput1 acceptance. Prior FramePool and PrintWindow failures remain open.\n",
        )?;
        source_snapshot
            .inputs
            .push(records::artifact(&root, &scope)?);
    }
    if frame_parity {
        let scope = inputs.join("frame-pool-scope.md");
        fs::write(
            &scope,
            "# P1 partial FramePool migration\n\n\
            Retain original caller-thread first-use initialization, hardware D3D11CreateDeviceAndSwapChain flags zero/R8G8B8A8, Create/BGRA/one-buffer, size-change full uninit/init, old-frame drain/Close, 2ms polling under 2000ms, timeout cache clone, mapped client ROI and conversion/cache clone before Unmap. Uninit retains cache/device/context/swapchain; final destruction releases them in source member order. Caught boolean failures and escaping WinRT exceptions preserve different partial-state behavior. Three optional property requests retain metadata order and native five-second completion-handler event wait; late delegates independently own state without accessing the capture owner.\n\n\
            Isolated original executable observes four fresh caller contexts. Rust runs in four separate processes (uninitialized/MTA/MTA-with-dispatcher/STA) against those exact original image bytes, with two sample calls, independent cache/result clones, retained pixels after native/window close, callback-event release, apartment/DPI evidence and GDI regression. The dispatcher case pumps only during harness shutdown after capture owners close. A separate twenty-cycle Rust test retains all image owners through capture/window/device and completed dispatcher shutdown, comparing the original dispatcher image bytes; this is not a repeated C++ source/fault comparison or complete GPU/system leak measurement.\n\n\
            Safety adaptations: checked geometry/layout/content bounds and compatible CopyResource descriptors; copy only initialized target BGRA rows into fallible CPU storage, excluding mapped padding. This additional target copy is counted, not advertised as removed work. Explicit factories avoid the projection global raw cache; SDK-style implicit MTA and broad DLL fallback are preserved. Factory ownership/cache behavior needs further lifetime/failure validation; old WGC unload AV remains unresolved.\n\n\
            Pseudo wrapper: shared original helper starts at construction, ensures before capture, restores on inactive and is fully destroyed before inner shutdown. Five isolated original image/state scenarios feed twenty Rust wrapper lifecycles. Source known-color failures remain failed and nonpassing even if byte/state diagnostics match; local alpha-zero black samples are not accepted as known-color capture. Foreground bits are recorded separately, with nonforeground required for minimized premises.\n\n\
            Pending: full repeated source/fault lifecycles, native failure/cleanup and initialization-exception interposition, queued-frame/timeout/cache failure matrices, resize/DPI/borders/secondary windows/permissions, complete pseudo-minimize wrapper/foreground and manager/selection chain. No complete backend, U/P1 exit, public SDK, P2 patch or P3 FreeThreaded/worker is accepted. All previous failures remain archived.\n",
        )?;
        source_snapshot
            .inputs
            .push(records::artifact(&root, &scope)?);
    }
    if frame_context {
        let scope = inputs.join("frame-pool-context-scope.md");
        fs::write(
            &scope,
            "# P1 original FramePool caller-context observation\n\n\
             Compile the fixed unmodified FramePoolScreencap class into an isolated executable. Run uninitialized, explicit MTA, MTA with current-thread dispatcher, and STA in four new processes; each executes exactly one original capture, without harness pumping/retry inside capture. Record initial/before/after/destruction/final COM type/qualifier, native owner identities, elapsed capture time, source image and known interior pixels. Dispatcher shutdown pumps until actual completion; only successful explicit RoInitialize references are balanced. The source's implicit process MTA is observed, never silently undone.\n\n\
             No Rust FramePool backend or public SDK is implemented by this command. C++/WinRT process factory state is isolated across contexts. This evidence establishes only this source's tested caller/runtime premise, not universal dispatcher requirements, full FramePool pixels/format/resize/timeout/cache/permission semantics, Rust parity, or P1 acceptance. All failed contexts and other historical failures remain distinct.\n",
        )?;
        source_snapshot
            .inputs
            .push(records::artifact(&root, &scope)?);
    }
    if screen_parity {
        let scope = inputs.join("screen-dc-scope.md");
        fs::write(
            &scope,
            "# P1 ScreenDC migration component scope\n\n\
             Fixed unpatched ScreenDCScreencap and ForegroundUtils/HwndUtils; U retains rcWork, source sleep requests, global DWORD cooldown and original asynchronous window adjustment. Foreground/geometry warning results do not abort capture. A null target is invalid; GetDC/ReleaseDC use the null screen owner and BitBlt uses the client screen origin. Conversion precedes native cleanup, with the same shared GetBitmapBits safety checks as window GDI. No P2 adjustment or P3 SDK/DIB behavior is introduced.\n\n\
             Compiled exact-original headers are observed synchronously: 11 continuous cooldown steps and two direct topmost cases; 16 geometry, three window-size, four style and four DPI cases. Overflow safety cases never execute undefined C++ arithmetic. Scripted native capture tests separately check warning continuation, client-left/top input, negative source origin, native/short-read/conversion failures, independent cleanup errors and unwind cleanup. These scripted boundaries are not real native failure acceptance.\n\n\
             Desktop test requests 20 paired original/Rust captures with exact known pixels, retained CPU outputs and real GDI counts after capture/window close. Fixture-only controls distinguish cold CreateWindow process initialization from capture accounting. Source-only before/after submission/presentation observations retain both images. A later formal run failed both-source/Rust and post-barrier source black-image assertions; those raw images and failures remain archived. The fixture now reuses a dedicated WM_PAINT/UI owner with joined shutdown, without compiling the PrintWindow backend in a ScreenDC-only target. Strict pixels and capture calls/waits remain unchanged, without retry. Outcomes must verify this changed fixture premise; a DwmFlush result alone does not prove readiness.\n\n\
             Thirteen instrumented exact-original native DC/bitmap/conversion control/failure/compound cases preserve reached faults, primary/cleanup errors and residual counts before separate fixture recovery. Three short-read/metadata/24-bit-DDB safety classifications, actual null/destroyed/zero-width client checks and a helper-only cold-process resource control are separate from visible-pixel assertions. No instrumented output is repaired or accepted as a real nondefault display.\n\n\
             Exact-commit Actions later observed source uniform gray with foreground true at cycle 1, subsequent Rust known pixels, and gray in original-only post-presentation control. Transition disabling alone still failed locally after wake and on Actions. Read-only point/root observations separately found another window over a new fixture before source capture; visible style and later foreground snapshots do not prove an unobscured source. The fixture now requests its own topmost root and disables transitions while hidden, retaining existing PrintWindow modes/observation prefixes. Point/root/foreground/cloak/geometry observations surround source/Rust samples. One awake local nineteen-check entry passed twenty strict paired pixel/resource/retention cycles plus the twenty-cycle source presentation control. This establishes only that controlled scene; it is not universal occlusion acceptance or a changed product capture/helper/wait. All previous failures remain preserved.\n\n\
             Occlusion, remaining native window-query/failure/cleanup matrices, asynchronous move/resize, nondefault depth, helper compatible-mode path, complete method/manager chains and full P1 acceptance remain outstanding. Native outcomes are platform/run specific.\n",
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
             Fixed unpatched GdiScreencap.{h,cpp} and HwndUtils.hpp::window_size/conversion order; parent U identity is retained in this run.\n\
             U-WIN-GDI-01 partial: controlled asymmetric BGR pixels, odd widths, conversion before GDI cleanup, retained CPU output and twenty cycles. Nondefault display depth remains unexecuted.\n\
             U-WIN-GDI-02 native scenarios: null/destroyed/empty/iconic HWND, DC/bitmap/select/copy/read/convert failures, each cleanup failure and a compound primary-plus-cleanup failure. Fault-instrumented original and the same Rust capture share real GDI boundary responses; compare order/results, report residuals before separate fixture recovery and verify CPU owner releases.\n\
             Safe defect observations: the instrumented reference initializes input for positive short-return/layout probes; Rust rejects incomplete reads and failed metadata queries. A real 24-bit DDB may be rejected at SelectObject on the current 32-bit device; this is not a nondefault-depth display capture.\n\
             Safety adaptations: checked arithmetic/narrowing and fallible initialized allocation; GetObjectW validates the original packed one-plane 32-bit DDB assumption; short GetBitmapBits reads and invalid SelectObject sentinels are rejected. These are not DIB modernization.\n\
             No manager/selection/forwarding or other backend case is closed. Actual native outcomes remain in the check logs; no complete GDI backend, U snapshot, P1 exit, public SDK, PR or P3 feature is asserted by this component run.\n",
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
             Fixed unpatched PrintWindowScreencap.{h,cpp}; the GDI regression scope is retained separately.\n\
             U-WIN-PRINT-01/02 partial: owned WM_PAINT/WM_PRINTCLIENT class and real PrintWindow(3), main/popup BGR conversion before cleanup, composition after both cleanups, negative/partial/outside/hidden/absent and zero-size popup scenes, twenty fixture lifecycles each, retained CPU ownership. Actual rendering message counts remain observable; no single message path is assumed. Four hundred compiled OpenCV/Rust rectangle boundaries preserve representable empty/clipped results.\n\
             A real child target inside a larger root also runs six scenes for twenty fixture lifecycles each: no root popup, visible/negative/outside root-owned popup, hidden and zero-width popup. Exact child pixels/dimensions, root != target, conversion/composition counts, cleanup and retained CPU output distinguish the child from its root. Hidden fixture setup verifies the resulting native visibility. The fixture independently owns its callback module; an explicit native close after Adapter destruction verifies code/context retention through class unregistration. Forgetting the guard retains that lease rather than unloading live callback code.\n\
             Twenty-nine ordinary native fault scenarios compare isolated exact original function bodies with Rust: main/popup client rectangle, DC/bitmap/selection/PrintWindow/read/conversion failures, each cleanup failure and compound primary/cleanup failures. The ledger retains both HWND resource sets, records residuals before separate fixture recovery and compares attempts/errors/CPU owners without treating that recovery as successful product cleanup. Six additional read-deselect/reselect/short-scanline scenarios classify safety adaptations; the instrumented short-read source input is fully defined. Successful GetLastError values remain raw observations, not guaranteed error contracts.\n\
             U-WIN-PRINT-03 partial: null/destroyed main and zero-size optional popup. Main/popup conversion exceptions in isolated original function bodies and the Rust converter boundary abort the whole operation. Real GDI counts before conversion/after unwind and exported image-owner counts remain distinct from the original internal Mat lifetime: a per-output allocator observes its main BGR allocation/release bytes, without changing the default allocator or observing input BGRA/other OpenCV allocations. Fault modes export no Mat referencing that observer; unexpected outstanding allocations fail and retain the module for late release. Eight query scenarios compare isolated original function bodies and Rust through real native query boundaries: normal/null root, null/root-equal popup, invalid/hidden popup and failure of either ClientToScreen query, including short-circuit order and main-image fallback. Full foreground/background/pseudo-minimize/window-state coverage remains pending.\n\
             The private PseudoMinimizeHelper and PrintWindow wrapper migration preserves synchronous ensure, 100ms polling, partial apply failures, foreground/target-loss observation, inactive restoration and join-before-stop restoration/re-minimization. Twenty-two exact original-body scripts use explicit fake scheduling and compare native calls, sleep requests, failure codes and state snapshots; they do not prove OS thread ownership or desktop pixels. Separate real-worker synchronization checks join order. Native fixtures now own distinct UI threads and real message loops, reading statistics and performing window/class shutdown on those owners, then joining before context/module release. Initial setup draining is bounded; partial construction and Adapter-first destruction check worker/module ownership. Layered/nonlayered stop/inactive and target-loss lifecycles each retain strict known pixels and CPU owners after shutdown. Earlier same-thread layered failures remain archived; independent pumped-owner controls correct the fixture without changing capture/helper calls, waits or conversion timing. Foreground recovery is a separate exact native case; verified missing foreground control is missing_environment, never a passing case, and other independent checks continue. No input injection or activation bypass is added. Earlier popup failures remain unresolved despite later non-reproduction. No helper/wrapper or PrintWindow exit is asserted.\n\
             Safety adaptation: temporarily deselect bitmap only around GetDIBits to meet its documented precondition, reselect before original conversion point, retain final restore/delete/delete/release. A real native selected-object witness verifies both boundaries. DWORD-aligned initialized u32 storage, checked CPU spans/scanline counts and signed geometry reject unsafe source assumptions; no CreateDIBSection/P3 migration is introduced.\n\
             An earlier formal native run returned a black original popup bitmap while the subsequent Rust capture rendered the fixture pattern. Same-bitmap selected/unselected/repeated/raw reads confirmed black contents; later query/flush controls and an exact earlier-fixture replay did not reproduce it and do not establish a fix. Preserve that unresolved parity failure independently of this run's check outcomes; no PrintWindow component exit is asserted.\n\
             No full backend, U snapshot, P1 exit, public SDK, manager, PR or later product feature is accepted by this record.\n",
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
    if frame_context {
        process.arg("-FramePoolContext");
    }
    if frame_parity {
        process.arg("-FramePoolParity");
    }
    if desktop_parity {
        process.arg("-DesktopDupParity");
    }
    if manager_parity {
        process.arg("-ManagerParity");
    }
    if pipewire_control {
        process.arg("-PipeWireControl");
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
    println!(
        "Native {} evidence archived at {}",
        if print_parity {
            "P1 PrintWindow component"
        } else if screen_parity {
            "P1 ScreenDC component"
        } else if frame_context {
            "P1 original FramePool caller context"
        } else if frame_parity {
            "P1 FramePool component"
        } else if desktop_parity {
            "P1 Desktop Duplication component"
        } else if manager_parity {
            "P1 screenshot-manager component"
        } else if pipewire_control {
            "P1 portable PipeWire consumer model (no Linux native acceptance)"
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
