// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! P1 first capture with a real asynchronous correction queued on a held owner.
#[path = "desktop_fixture_tests.rs"]
mod preparation;

use crate::backends::windows::desktop_dup_window::DesktopDupWindowCapture;
use crate::backends::windows::parity_adapter::{Adapter, Image};
use crate::platform::windows::window_api::{Point, SystemWindow, Window, WindowApi};
use std::{
    error::Error,
    fs::OpenOptions,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

fn caller_thread() -> u64 {
    // SAFETY: Scalar native identity query; no pointers or ownership transfer.
    u64::from(unsafe { crate::platform::windows::desktop_dup::native::GetCurrentThreadId() })
}

fn resource_count(adapter: &Adapter) -> Result<u32, Box<dyn Error>> {
    crate::platform::windows::test_library::checked_gdi_count(|| adapter.objects())
        .map_err(|error| format!("GetGuiResources failed: {error}").into())
}

fn log_resource_stage(cycle: u32, side: &str, stage: &str, objects: u32) {
    // Intermediate counts may include paint work on either UI owner. Only the
    // existing before-create/after-both-joins rows are the strict balance gate.
    eprintln!(
        "desktop_pending_resource cycle={cycle} side={side} stage={stage} caller={} gdi={objects}",
        caller_thread()
    );
}

fn save(root: &Path, name: &str, image: &Image) -> Result<(), Box<dyn Error>> {
    let mut pixels = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(format!("{name}.bgr")))?;
    pixels.write_all(image.bytes())?;
    let mut layout = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(format!("{name}.txt")))?;
    let (width, height) = image.dimensions();
    writeln!(
        layout,
        "{width}\t{height}\t{}\t{}",
        image.stride(),
        image.bytes().len()
    )?;
    Ok(())
}
fn client(row: &[i64; 31]) -> [i64; 4] {
    [
        row[16] + row[12],
        row[17] + row[13],
        row[16] + row[14],
        row[17] + row[15],
    ]
}
fn rectangle(window: Window) -> Result<[i32; 4], Box<dyn Error>> {
    // These are native geometry queries on the calling capture thread; no
    // fixture owner command, WindowFromPoint or synchronous message is used.
    let rect = SystemWindow
        .client_rect(window)
        .map_err(|e| format!("GetClientRect {e}"))?;
    let point = SystemWindow
        .origin(
            window,
            Point {
                x: rect.left,
                y: rect.top,
            },
        )
        .map_err(|e| format!("ClientToScreen {e}"))?;
    let width = rect.right.checked_sub(rect.left).ok_or("width overflow")?;
    let height = rect.bottom.checked_sub(rect.top).ok_or("height overflow")?;
    Ok([
        point.x,
        point.y,
        point.x.checked_add(width).ok_or("x overflow")?,
        point.y.checked_add(height).ok_or("y overflow")?,
    ])
}

#[test]
#[ignore = "requires an explicitly enabled native visible desktop and rendering adapter"]
fn balances_process_gdi_after_both_render_workers_join_without_capturing()
-> Result<(), Box<dyn Error>> {
    let root = std::env::var_os("MFS_DESKTOP_RESOURCE_FIXTURE_ROOT")
        .ok_or("missing render-resource evidence directory")?;
    let root = std::path::PathBuf::from(root);
    std::fs::create_dir_all(&root)?;
    let mut records = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("fixtures.tsv"))?;
    writeln!(records, "cycle\tstage\tvalues")?;
    let adapter = Adapter::load()?;
    // Match the main test's one initial geometry fixture before accounting.
    let mut warm = adapter.render_fixture(192, 128)?;
    warm.geometry([0; 6])?;
    warm.close()?;
    for cycle in 0..20 {
        let baseline = resource_count(&adapter)?;
        let caller = caller_thread();
        writeln!(
            records,
            "{cycle}\tbaseline\t{:?}",
            [u64::from(baseline), caller]
        )?;
        let mut target = adapter.render_fixture(192, 128)?;
        let mut background = adapter.render_fixture(32, 32)?;
        let target_window = target.hwnd();
        let update_window = background.hwnd();
        let target_threads = target.thread_info()?;
        let update_threads = background.thread_info()?;
        writeln!(
            records,
            "{cycle}\towners\t{:?}",
            [
                target_window as usize as u64,
                update_window as usize as u64,
                target_threads[0],
                update_threads[0],
                caller_thread(),
            ]
        )?;
        assert_eq!(target_threads[1], caller);
        assert_eq!(update_threads[1], caller);
        assert_ne!(target_threads[0], update_threads[0]);
        background.start_inside_desktop_updates()?;
        let updates = background.desktop_update_info()?;
        writeln!(records, "{cycle}\tupdates\t{updates:?}")?;
        assert!(updates[1] > 0 && updates[2] == 0);
        background.close()?;
        target.close()?;
        let target_valid = SystemWindow.valid(target_window);
        let update_valid = SystemWindow.valid(update_window);
        let objects = resource_count(&adapter)?;
        writeln!(
            records,
            "{cycle}\tclosed\t{:?}",
            [
                u64::from(objects),
                caller_thread(),
                u64::from(target_valid),
                u64::from(update_valid),
            ]
        )?;
        records.flush()?;
        assert!(!target_valid && !update_valid);
        assert_eq!(objects, baseline, "fixture counts after both owner joins");
    }
    println!(
        "desktop_resource_fixture_cases=20 joined_workers=40 invalid_windows=40 capture_calls=0 passed"
    );
    Ok(())
}

#[test]
#[ignore = "requires an explicitly enabled native visible desktop and original adapter"]
fn captures_first_client_before_queued_adjustment_and_after_exactly_one_move()
-> Result<(), Box<dyn Error>> {
    let root =
        std::env::var_os("MFS_DESKTOP_PENDING_ROOT").ok_or("missing pending evidence directory")?;
    let root = std::path::PathBuf::from(root);
    std::fs::create_dir_all(&root)?;
    let mut records = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("pending.tsv"))?;
    writeln!(records, "cycle\tside\tstage\tvalues")?;
    let adapter = Adapter::load()?;
    let mut fixture = adapter.render_fixture(192, 128)?;
    let initial = fixture.geometry([0; 6])?;
    fixture.close()?;
    let monitor: [i64; 4] = initial[..4].try_into()?;
    let work: [i64; 4] = initial[4..8].try_into()?;
    if work[2] - work[0] < 192 || work[3] - work[1] < 128 {
        return Err("missing_environment: work area cannot contain the 192x128 fixture".into());
    }
    let mut before = [work[0], work[1], work[0] + 192, work[1] + 128];
    if work[3] < monitor[3] {
        before[1] = monitor[3] - 128;
        before[3] = monitor[3];
    } else if work[1] > monitor[1] {
        before[1] = monitor[1];
        before[3] = monitor[1] + 128;
    } else if work[2] < monitor[2] {
        before[0] = monitor[2] - 192;
        before[2] = monitor[2];
    } else if work[0] > monitor[0] {
        before[0] = monitor[0];
        before[2] = monitor[0] + 192;
    } else {
        return Err(
            "missing_environment: real monitor/work bounds do not require correction".into(),
        );
    }
    let left = before[0].clamp(work[0], work[2] - 192);
    let top = before[1].clamp(work[1], work[3] - 128);
    let after = [left, top, left + 192, top + 128];
    assert_ne!(before, after);
    writeln!(records, "-1\tfixture\tmonitor\t{monitor:?}")?;
    writeln!(records, "-1\tfixture\twork\t{work:?}")?;
    writeln!(records, "-1\tfixture\tbefore\t{before:?}")?;
    writeln!(records, "-1\tfixture\tafter\t{after:?}")?;
    writeln!(records, "-1\tfixture\tresources\t[2]")?;
    records.flush()?;
    let mut updates = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("updates.tsv"))?;
    writeln!(updates, "cycle\tside\tstage\tvalues")?;
    let mut resources = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("resources.tsv"))?;
    writeln!(resources, "cycle\tside\tstage\tvalues")?;
    let overlaps =
        |a: [i64; 4], b: [i64; 4]| a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
    let update_rect = [
        [work[0], work[1]],
        [work[2] - 32, work[1]],
        [work[0], work[3] - 32],
        [work[2] - 32, work[3] - 32],
    ]
    .into_iter()
    .map(|[x, y]| [x, y, x + 32, y + 32])
    .find(|r| !overlaps(*r, before) && !overlaps(*r, after))
    .ok_or("missing_environment: no disjoint update fixture location")?;
    let mut held = Vec::new();
    for cycle in 0..20 {
        for side in ["source", "rust"] {
            // Both owned UI workers from the preceding side have joined.
            // Sample before starting either renderer, never during timer paint.
            let baseline = resource_count(&adapter)?;
            log_resource_stage(cycle, side, "before-create", baseline);
            let baseline_caller = caller_thread();
            let mut fixture = adapter.render_fixture(192, 128)?;
            let mut background = adapter.render_fixture(32, 32)?;
            let target_window = fixture.hwnd();
            let update_window = background.hwnd();
            let target_threads = fixture.thread_info()?;
            let update_threads = background.thread_info()?;
            writeln!(
                resources,
                "{cycle}\t{side}\tbaseline\t{:?}",
                [u64::from(baseline), baseline_caller]
            )?;
            writeln!(
                resources,
                "{cycle}\t{side}\towners\t{:?}",
                [
                    target_window as usize as u64,
                    update_window as usize as u64,
                    target_threads[0],
                    update_threads[0],
                    caller_thread(),
                ]
            )?;
            resources.flush()?;
            let update_geometry =
                background.geometry([1, 0x9000_0000, update_rect[0], update_rect[1], 32, 32])?;
            assert_eq!(client(&update_geometry), update_rect);
            assert_eq!(update_geometry[..8], initial[..8]);
            background.start_inside_desktop_updates()?;
            let update_threads = background.thread_info()?;
            assert_ne!(
                update_threads[0],
                fixture.thread_info()?[0],
                "independent target/update UI owners"
            );
            let update_before = background.desktop_update_info()?;
            writeln!(updates, "{cycle}\t{side}\tgeometry\t{update_geometry:?}")?;
            writeln!(updates, "{cycle}\t{side}\tthreads\t{update_threads:?}")?;
            writeln!(updates, "{cycle}\t{side}\tbefore\t{update_before:?}")?;
            let setup = fixture.geometry([1, 0x10cf_0000, before[0], before[1], 192, 128])?;
            assert_eq!(client(&setup), before, "actual initial client geometry");
            let presented = fixture.geometry([2, 0, 0, 0, 0, 0])?;
            writeln!(records, "{cycle}\t{side}\tsetup\t{presented:?}")?;
            records.flush()?;
            assert_eq!(adapter.screen_present()?, [1, 0]);
            fixture.activate()?;
            let visible = adapter.screen_window_observation(fixture.hwnd())?;
            writeln!(records, "{cycle}\t{side}\tvisible\t{visible:?}")?;
            assert_eq!(visible[..3], [1, 1, 0]);
            assert_eq!(visible[3], visible[4], "actual foreground before hold");
            assert_eq!(visible[6], visible[7], "target is visible at its center");
            assert_eq!(visible[8..10], [0, 0], "not cloaked");
            log_resource_stage(cycle, side, "before-hold", resource_count(&adapter)?);
            let gate = fixture.hold_queue()?;
            let entered = gate.state()?;
            writeln!(records, "{cycle}\t{side}\tentered\t{entered:?}")?;
            records.flush()?;
            assert_eq!(
                SystemWindow.foreground(),
                gate.hwnd(),
                "foreground must not require a synchronous owner call"
            );
            let mut capture = DesktopDupWindowCapture::new(gate.hwnd());
            let mut original = if side == "source" {
                Some(adapter.desktop_pending_source(gate.hwnd())?)
            } else {
                None
            };
            let image = if side == "source" {
                original
                    .as_mut()
                    .unwrap()
                    .capture()
                    .map_err(|e| format!("original pending capture {e}"))?
            } else {
                let mut converter = adapter.converter(0);
                let attempt = capture.capture(&mut converter);
                assert!(attempt.evidence.sample_count >= 1);
                assert_eq!(attempt.target_copy_bytes, 73728);
                let coordination = attempt.coordination.ok_or("missing coordination")?;
                assert!(coordination.foreground && coordination.geometry.is_ok());
                attempt
                    .image
                    .map_err(|e| format!("Rust pending capture {e:?}"))?
            };
            let pending = gate.state()?;
            let actual = rectangle(gate.hwnd())?;
            writeln!(records, "{cycle}\t{side}\tpending\t{pending:?}")?;
            writeln!(records, "{cycle}\t{side}\tinitial-client\t{actual:?}")?;
            save(&root, &format!("{cycle}-{side}-pending"), &image)?;
            assert_eq!(
                pending, entered,
                "owner must still be held after first capture"
            );
            assert_eq!(
                actual.map(i64::from),
                before,
                "no correction consumed before first capture"
            );
            assert_eq!(image.dimensions(), (192, 128));
            assert_eq!(first_nonfixture_pixel(&image), None, "first pending pixels");
            log_resource_stage(cycle, side, "first-capture", resource_count(&adapter)?);
            let released = gate.release()?;
            writeln!(records, "{cycle}\t{side}\treleased\t{released:?}")?;
            assert_eq!(released[..5], [1, 1, 1, 1, 0]);
            // Wait only after the initial capture. This proves the real queued
            // request takes effect; no capture is retried or preceded by a
            // separate geometry helper invocation.
            let deadline = Instant::now() + Duration::from_secs(3);
            let completed = loop {
                let row = fixture.geometry([0; 6])?;
                if client(&row) == after {
                    break row;
                }
                if Instant::now() >= deadline {
                    return Err(format!("queued correction not consumed: {row:?}").into());
                }
                std::thread::sleep(Duration::from_millis(1));
            };
            writeln!(records, "{cycle}\t{side}\tcompleted\t{completed:?}")?;
            assert_eq!(
                completed[20],
                setup[20] + 1,
                "exactly one real movement message"
            );
            assert_eq!(
                completed[21..25],
                [
                    setup[8] + after[0] - before[0],
                    setup[9] + after[1] - before[1],
                    setup[10] - setup[8],
                    setup[11] - setup[9]
                ]
            );
            assert_eq!(
                completed[25] & 0x14,
                0x14,
                "native NOZORDER/NOACTIVATE flags"
            );
            log_resource_stage(cycle, side, "corrected", resource_count(&adapter)?);
            fixture.geometry([2, 0, 0, 0, 0, 0])?;
            assert_eq!(adapter.screen_present()?, [1, 0]);
            let final_image = if side == "source" {
                original
                    .as_mut()
                    .unwrap()
                    .capture()
                    .map_err(|e| format!("original completed capture {e}"))?
            } else {
                let mut converter = adapter.converter(0);
                let attempt = capture.capture(&mut converter);
                assert!(attempt.evidence.sample_count >= 1);
                assert_eq!(attempt.target_copy_bytes, 73728);
                assert!(
                    attempt
                        .coordination
                        .ok_or("missing final coordination")?
                        .geometry
                        .is_ok()
                );
                attempt
                    .image
                    .map_err(|e| format!("Rust completed capture {e:?}"))?
            };
            save(&root, &format!("{cycle}-{side}-completed"), &final_image)?;
            assert_eq!(final_image.dimensions(), (192, 128));
            assert_eq!(
                first_nonfixture_pixel(&final_image),
                None,
                "completed pixels"
            );
            log_resource_stage(cycle, side, "second-capture", resource_count(&adapter)?);
            let final_geometry = fixture.geometry([0; 6])?;
            writeln!(records, "{cycle}\t{side}\tfinal\t{final_geometry:?}")?;
            assert_eq!(final_geometry[20], completed[20], "no repeated adjustment");
            drop(original);
            drop(capture);
            log_resource_stage(cycle, side, "capture-dropped", resource_count(&adapter)?);
            let update_after = background.desktop_update_info()?;
            assert_eq!(update_after[0], update_before[0]);
            assert!(update_after[1] > update_before[1] && update_after[2] == 0);
            writeln!(updates, "{cycle}\t{side}\tafter\t{update_after:?}")?;
            background.close()?;
            log_resource_stage(cycle, side, "update-joined", resource_count(&adapter)?);
            writeln!(updates, "{cycle}\t{side}\tclosed\t[1]")?;
            updates.flush()?;
            fixture.close()?;
            // close() checks native cleanup and joins each worker. Verify the
            // actual HWNDs are gone before the matching process-wide snapshot.
            let target_valid = SystemWindow.valid(target_window);
            let update_valid = SystemWindow.valid(update_window);
            let objects = resource_count(&adapter)?;
            log_resource_stage(cycle, side, "both-joined", objects);
            writeln!(
                resources,
                "{cycle}\t{side}\tclosed\t{:?}",
                [
                    u64::from(objects),
                    caller_thread(),
                    u64::from(target_valid),
                    u64::from(update_valid),
                ]
            )?;
            resources.flush()?;
            writeln!(records, "{cycle}\t{side}\tgdi\t{:?}", [baseline, objects])?;
            assert!(!target_valid && !update_valid, "owned windows must be gone");
            assert_eq!(
                objects, baseline,
                "balanced GDI resources after UI workers join"
            );
            writeln!(records, "{cycle}\t{side}\tclosed\t[1]")?;
            held.push((format!("{cycle}-{side}-pending-held"), image));
            held.push((format!("{cycle}-{side}-completed-held"), final_image));
            records.flush()?;
        }
    }
    for (name, image) in &held {
        save(&root, name, image)?;
        assert_eq!(
            first_nonfixture_pixel(image),
            None,
            "post-close retained pixels"
        );
    }
    println!(
        "desktop_pending_cases=40 first_captures=40 final_captures=40 held=80 queue_timeouts=0 independent_update_owners=40 quiescent_counts=40 full_P1_acceptance=0 passed"
    );
    Ok(())
}

fn first_nonfixture_pixel(image: &Image) -> Option<usize> {
    for y in 0..128 {
        for x in 0..192 {
            let expected = if x < 64 {
                [31, 83, 167]
            } else if y < 64 {
                [185, 77, 39]
            } else {
                [59, 226, 129]
            };
            let at = y * image.stride() + x * 3;
            if image.bytes()[at..at + 3] != expected {
                return Some(at);
            }
        }
    }
    None
}

#[test]
#[ignore = "requires the actual original DesktopDupWindow acceptance adapter, no visible desktop"]
fn consumes_original_owner_and_rejects_null_window_without_frames() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    assert_eq!(adapter.image_owners()?, 0);
    for _ in 0..20 {
        let mut source = adapter.desktop_pending_source(std::ptr::null_mut())?;
        assert!(matches!(source.capture(), Err(1)));
        drop(source);
        assert_eq!(adapter.image_owners()?, 0);
    }
    println!(
        "DesktopDupWindow original adapter owners=20 null_window_calls=20 failure_outputs_empty=20 owners_destroyed=20 native_frames=0 passed"
    );
    Ok(())
}
