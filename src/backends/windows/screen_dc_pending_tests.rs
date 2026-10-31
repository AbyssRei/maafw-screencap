// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! P1 first capture with a real asynchronous correction queued on a held owner.
use super::*;
use crate::backends::windows::parity_adapter::{Adapter, Image};
use std::{
    error::Error,
    fs::OpenOptions,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

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
#[ignore = "requires an explicitly enabled native visible desktop and original adapter"]
fn matches_first_capture_before_queued_window_adjustment_is_consumed() -> Result<(), Box<dyn Error>>
{
    let root = std::env::var_os("MFS_SCREEN_PENDING_EVIDENCE")
        .ok_or("missing pending evidence directory")?;
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
    records.flush()?;
    let mut held = Vec::new();
    for cycle in 0..20 {
        for side in ["source", "rust"] {
            let mut fixture = adapter.render_fixture(192, 128)?;
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
            let baseline = adapter.objects();
            let gate = fixture.hold_queue()?;
            let entered = gate.state()?;
            writeln!(records, "{cycle}\t{side}\tentered\t{entered:?}")?;
            records.flush()?;
            assert_eq!(
                SystemWindow.foreground(),
                gate.hwnd(),
                "foreground must not require a synchronous owner call"
            );
            let mut capture = ScreenDcCapture::new(gate.hwnd());
            let image = if side == "source" {
                adapter
                    .screen_reference(gate.hwnd())
                    .map_err(|e| format!("original pending capture {e}"))?
            } else {
                let mut converter = adapter.converter(0);
                let attempt = capture.capture(&mut converter);
                assert!(attempt.cleanup.0.iter().all(Option::is_none));
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
            fixture.geometry([2, 0, 0, 0, 0, 0])?;
            assert_eq!(adapter.screen_present()?, [1, 0]);
            let final_image = if side == "source" {
                adapter
                    .screen_reference(fixture.hwnd())
                    .map_err(|e| format!("original completed capture {e}"))?
            } else {
                let mut converter = adapter.converter(0);
                let attempt = capture.capture(&mut converter);
                assert!(attempt.cleanup.0.iter().all(Option::is_none));
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
            let final_geometry = fixture.geometry([0; 6])?;
            writeln!(records, "{cycle}\t{side}\tfinal\t{final_geometry:?}")?;
            assert_eq!(final_geometry[20], completed[20], "no repeated adjustment");
            let objects = adapter.objects();
            writeln!(records, "{cycle}\t{side}\tgdi\t{:?}", [baseline, objects])?;
            assert_eq!(objects, baseline, "balanced GDI resources");
            fixture.close()?;
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
        "screen_pending_cases=40 first_captures=40 final_captures=40 held=80 queue_timeouts=0"
    );
    Ok(())
}
