// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Zero-capture diagnostic of the pending test's full pre-hold fixture path.
use super::{
    Adapter, SystemWindow, WindowApi, caller_thread, client, finish_fixture_initialization,
    resource_count,
};
use std::{error::Error, fmt::Debug, fs::OpenOptions, io::Write, path::PathBuf};

fn observe(
    records: &mut impl Write,
    adapter: &Adapter,
    cycle: u32,
    stage: &str,
    values: impl Debug,
) -> Result<u32, Box<dyn Error>> {
    let count = resource_count(adapter)?;
    writeln!(
        records,
        "{cycle}\t{stage}\t{count}\t{}\t{values:?}",
        caller_thread()
    )?;
    records.flush()?;
    Ok(count)
}

#[test]
#[ignore = "requires an explicitly enabled native visible desktop and rendering adapter"]
fn observes_full_pre_capture_fixture_counts_and_checks_both_worker_joins()
-> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(
        std::env::var_os("MFS_DESKTOP_PREPARATION_ROOT")
            .ok_or("missing full fixture preparation evidence directory")?,
    );
    std::fs::create_dir_all(&root)?;
    let mut records = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("preparation.tsv"))?;
    writeln!(records, "cycle\tstage\tgdi\tcaller\tvalues")?;
    let adapter = Adapter::load()?;
    // Use the capture test's single matching geometry/style initialization.
    // Its checked close and observed counts are explicit, without a capture.
    let mut fixture = adapter.render_fixture(192, 128)?;
    let initial = fixture.geometry([0; 6])?;
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
    finish_fixture_initialization(&adapter, &mut fixture, before, &root)?;
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
    let mut layout = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("layout.tsv"))?;
    writeln!(layout, "name\tvalues")?;
    writeln!(layout, "initial\t{initial:?}")?;
    for (name, rect) in [
        ("monitor", monitor),
        ("work", work),
        ("before", before),
        ("after", after),
        ("update", update_rect),
    ] {
        writeln!(layout, "{name}\t{rect:?}")?;
    }
    layout.flush()?;
    let mut mismatches = Vec::new();
    // Twenty predetermined independent fixture lifecycles, never retries of
    // capture or until equality. Retain every mismatch and fail the whole test.
    for cycle in 0..20 {
        let baseline = observe(&mut records, &adapter, cycle, "baseline", [0_u64; 0])?;
        let caller = caller_thread();
        let mut target = adapter.render_fixture(192, 128)?;
        observe(&mut records, &adapter, cycle, "target-created", [0_u64; 0])?;
        let mut background = adapter.render_fixture(32, 32)?;
        observe(
            &mut records,
            &adapter,
            cycle,
            "background-created",
            [0_u64; 0],
        )?;
        let target_window = target.hwnd();
        let update_window = background.hwnd();
        let preparation = (|| -> Result<(), Box<dyn Error>> {
            let target_threads = target.thread_info()?;
            let update_threads = background.thread_info()?;
            observe(
                &mut records,
                &adapter,
                cycle,
                "owners",
                [
                    target_window as usize as u64,
                    update_window as usize as u64,
                    target_threads[0],
                    update_threads[0],
                    target_threads[1],
                    update_threads[1],
                ],
            )?;
            if target_threads[1] != caller
                || update_threads[1] != caller
                || target_threads[0] == update_threads[0]
            {
                return Err("independent target/update UI owners required".into());
            }
            let update_geometry =
                background.geometry([1, 0x9000_0000, update_rect[0], update_rect[1], 32, 32])?;
            observe(
                &mut records,
                &adapter,
                cycle,
                "background-geometry",
                update_geometry,
            )?;
            if client(&update_geometry) != update_rect || update_geometry[..8] != initial[..8] {
                return Err("actual disjoint update fixture geometry differs".into());
            }
            background.start_inside_desktop_updates()?;
            let updates = background.desktop_update_info()?;
            observe(&mut records, &adapter, cycle, "updates-started", updates)?;
            if updates[1] == 0 || updates[2] != 0 {
                return Err("independent update fixture did not produce in-target updates".into());
            }
            let setup = target.geometry([1, 0x10cf_0000, before[0], before[1], 192, 128])?;
            observe(&mut records, &adapter, cycle, "target-style", setup)?;
            if client(&setup) != before {
                return Err("actual initial target client geometry differs".into());
            }
            let presented = target.geometry([2, 0, 0, 0, 0, 0])?;
            observe(&mut records, &adapter, cycle, "target-repaint", presented)?;
            let presentation = adapter.screen_present()?;
            observe(&mut records, &adapter, cycle, "presented", presentation)?;
            if presentation != [1, 0] {
                return Err("native presentation barrier failed".into());
            }
            let activated = target.activate();
            observe(
                &mut records,
                &adapter,
                cycle,
                "activated",
                [u32::from(activated.is_ok())],
            )?;
            activated?;
            let visible = adapter.screen_window_observation(target_window)?;
            observe(&mut records, &adapter, cycle, "visible", visible)?;
            if visible[..3] != [1, 1, 0]
                || visible[3] != visible[4]
                || visible[6] != visible[7]
                || visible[8..10] != [0, 0]
            {
                return Err("missing_environment: target lacks the capture test's visible foreground premise".into());
            }
            Ok(())
        })();
        // Even a failed preparation gets both checked close attempts and
        // invalid-HWND observations before its original error is returned.
        let background_close = background.close();
        let background_observation = observe(
            &mut records,
            &adapter,
            cycle,
            "background-close",
            [u32::from(background_close.is_ok())],
        );
        let target_close = target.close();
        let target_valid = SystemWindow.valid(target_window);
        let update_valid = SystemWindow.valid(update_window);
        let final_observation = observe(
            &mut records,
            &adapter,
            cycle,
            "closed",
            [
                u32::from(target_close.is_ok()),
                u32::from(target_valid),
                u32::from(update_valid),
            ],
        );
        let mut errors = Vec::new();
        for (stage, result) in [
            ("preparation", preparation),
            ("background-close", background_close),
            ("target-close", target_close),
            ("background-observation", background_observation.map(|_| ())),
        ] {
            if let Err(error) = result {
                errors.push(format!("{stage}: {error}"));
            }
        }
        if target_valid || update_valid {
            errors.push("a fixture HWND remains valid after close".into());
        }
        match final_observation {
            Ok(final_count) if final_count != baseline => {
                mismatches.push((cycle, baseline, final_count));
            }
            Ok(_) => {}
            Err(error) => errors.push(format!("closed-observation: {error}")),
        }
        if !errors.is_empty() {
            return Err(format!(
                "full fixture cycle={cycle}: {}; resource_mismatches={mismatches:?}",
                errors.join("; ")
            )
            .into());
        }
    }
    if !mismatches.is_empty() {
        return Err(format!("full fixture resource mismatches: {mismatches:?}").into());
    }
    println!(
        "desktop_preparation_cases=20 joined_workers=40 invalid_windows=40 resource_mismatches=0 capture_calls=0 initialized_fixture=1 full_P1_acceptance=0 passed"
    );
    Ok(())
}
