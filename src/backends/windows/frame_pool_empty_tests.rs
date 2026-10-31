// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual zero-client geometry introduced by the UI owner after a real Map.
use super::*;
use crate::backends::windows::parity_adapter::{Adapter, Image};
use std::{cell::RefCell, error::Error as StdError, fs::OpenOptions, io::Write};

#[test]
#[ignore = "explicit original/Rust real zero-client mapped capture and records required"]
fn clears_warm_cache_on_real_zero_client_and_recovers_without_old_pixels()
-> Result<(), Box<dyn StdError>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_FRAMEPOOL_EMPTY_ROOT").ok_or("missing empty-client root")?,
    );
    let directory = root.join("rust");
    std::fs::create_dir(&directory)?;
    let prepare_restore =
        std::env::var("MFS_FRAMEPOOL_EMPTY_RESTORE_BARRIER").is_ok_and(|v| v == "1");
    let mut presentations = if prepare_restore {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("restore-presentations.tsv"))?;
        writeln!(file, "cycle\tbefore\tafter\towner_timer_errors")?;
        Some(file)
    } else {
        None
    };
    let mut rows = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("empty.tsv"))?;
    writeln!(
        rows,
        "cycle\tcase\tphase\toutcome\torder\tclient_width\tclient_height\tmarker\tcaller_thread\tfixture_thread"
    )?;
    let mut states = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("rust-state.tsv"))?;
    writeln!(
        states,
        "cycle\tphase\tpitch\tcopied\tconversions\tclones\tcache_present\tnative_slots\towners\tcompletion_live\tclose_errors"
    )?;
    let mut geometry = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("geometry.tsv"))?;
    writeln!(
        geometry,
        "cycle\tphase\tclient_width\tclient_height\touter_width\touter_height\tstyle\towner_timer_errors"
    )?;
    let mut finals = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("final.tsv"))?;
    writeln!(finals, "cycle\theld\towners\tcompletion_live\tclose_errors")?;
    let mut apartment = host::APTTYPE::default();
    let mut qualifier = host::APTTYPEQUALIFIER::default();
    // SAFETY: This fresh caller owns exactly one successful MTA reference.
    assert_eq!(
        unsafe { host::CoGetApartmentType(&mut apartment, &mut qualifier) },
        host::CO_E_NOTINITIALIZED
    );
    unsafe { host::RoInitialize(host::RO_INIT_MULTITHREADED).ok()? };
    let owned = HostInitialization(true);
    let adapter = Adapter::load()?;
    assert_eq!(adapter.image_owners()?, 0);
    let drop_errors = DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire);
    let mut held = Vec::new();
    held.try_reserve_exact(60)?;
    for cycle in 0..20_u32 {
        let case = cycle % 3;
        let dimensions = match case {
            0 => (0, 64),
            1 => (96, 0),
            _ => (0, 0),
        };
        let mut fixture = adapter.render_fixture(96, 64)?;
        fixture.geometry([1, i64::from(0x9000_0000_u32), 100, 100, 96, 64])?;
        fixture.start_inside_desktop_updates()?;
        let fixture = Rc::new(fixture);
        let owner = fixture.thread_info()?;
        let mut capture: FramePoolCapture<Image> = FramePoolCapture::new(fixture.hwnd());
        let mut converter = adapter.converter(0);
        for phase in ["warm", "zero", "recovery"] {
            let zero = phase == "zero";
            let snapshot = Rc::new(RefCell::new(None));
            let guard = if zero {
                let fixture = Rc::clone(&fixture);
                let output = Rc::clone(&snapshot);
                query_probe::Guard::before_client(Box::new(move || {
                    let actual = fixture
                        .geometry([
                            3,
                            i64::from(0x9080_0000_u32),
                            100,
                            100,
                            dimensions.0,
                            dimensions.1,
                        ])
                        .expect("actual UI zero-client resize");
                    assert_eq!((actual[14], actual[15]), dimensions);
                    assert!(actual[10] > actual[8] && actual[11] > actual[9]);
                    *output.borrow_mut() = Some(actual);
                }))
            } else {
                if phase == "recovery" {
                    fixture.geometry([1, i64::from(0x9000_0000_u32), 100, 100, 96, 64])?;
                    if let Some(presentations) = presentations.as_mut() {
                        fixture.geometry([2, 0, 0, 0, 0, 0])?;
                        let mut update = fixture.desktop_update_info()?;
                        assert!(update[1] > 0 && update[2] == 0);
                        let before = update[1];
                        let target = before.checked_add(2).ok_or("presentation count overflow")?;
                        let deadline = Instant::now() + Duration::from_secs(2);
                        while update[1] < target {
                            assert!(Instant::now() < deadline, "restore presentation deadline");
                            std::thread::sleep(Duration::from_millis(5));
                            update = fixture.desktop_update_info()?;
                            assert_eq!(update[2], 0);
                        }
                        writeln!(presentations, "{cycle}\t{before}\t{}\t0", update[1])?;
                        presentations.flush()?;
                    }
                }
                query_probe::Guard::begin(0)
            };
            let attempt = capture.capture(&mut converter);
            let trace = guard.trace();
            drop(guard);
            assert_eq!(trace.order, 123);
            assert_eq!((trace.win32, trace.dwm), (0, 0));
            assert_eq!(
                trace.client,
                Some(if zero {
                    (i32::try_from(dimensions.0)?, i32::try_from(dimensions.1)?)
                } else {
                    (96, 64)
                })
            );
            assert!(attempt.evidence.polls > 0 && attempt.evidence.mapped_pitch >= 384);
            assert!(
                !attempt.evidence.replayed && attempt.evidence.cleanup.iter().all(Option::is_none)
            );
            assert!(
                capture.pool.is_some()
                    && capture.session.is_some()
                    && capture.item.is_some()
                    && capture.readable.is_some()
            );
            let image = if zero {
                assert!(matches!(attempt.image, Err(Error::EmptySample)));
                assert!(capture.cached.is_none());
                assert_eq!(attempt.evidence.target_copy_bytes, 0);
                None
            } else {
                assert_eq!(attempt.evidence.target_copy_bytes, 24576);
                Some(
                    attempt
                        .image
                        .map_err(|e| format!("empty-client cycle={cycle} phase={phase}: {e:?}"))?,
                )
            };
            let marker = image.as_ref().map_or(-1, queries::known);
            let name = format!("{cycle}-{phase}");
            queries::save(&directory, &name, image.as_ref())?;
            held.push((name, image));
            let count = held.iter().filter(|(_, image)| image.is_some()).count();
            let conversions = if phase == "recovery" { 2 } else { 1 };
            assert_eq!(
                (converter.conversions, converter.clones),
                (conversions, conversions)
            );
            assert_eq!(
                adapter.image_owners()?,
                u64::try_from(count + usize::from(!zero))?
            );
            assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
            let size = trace.client.unwrap();
            writeln!(
                rows,
                "{cycle}\t{case}\t{phase}\t{}\t123\t{}\t{}\t{marker}\t{}\t{}",
                if zero { 2 } else { 1 },
                size.0,
                size.1,
                owner[1],
                owner[0]
            )?;
            writeln!(
                states,
                "{cycle}\t{phase}\t{}\t{}\t{conversions}\t{conversions}\t{}\t1\t{}\t0\t0",
                attempt.evidence.mapped_pitch,
                attempt.evidence.target_copy_bytes,
                usize::from(!zero),
                count + usize::from(!zero)
            )?;
            let native = snapshot
                .borrow()
                .unwrap_or(fixture.geometry([0, 0, 0, 0, 0, 0])?);
            let updates = fixture.desktop_update_info()?;
            assert_eq!(updates[2], 0);
            writeln!(
                geometry,
                "{cycle}\t{phase}\t{}\t{}\t{}\t{}\t{}\t0",
                native[14],
                native[15],
                native[10] - native[8],
                native[11] - native[9],
                native[18]
            )?;
            rows.flush()?;
            states.flush()?;
            geometry.flush()?;
        }
        assert!(capture.uninit().cleanup.iter().all(Option::is_none));
        drop(capture);
        Rc::try_unwrap(fixture)
            .map_err(|_| "zero-client fixture still borrowed")?
            .close()?;
        let count = usize::try_from(cycle + 1)? * 2;
        assert_eq!(adapter.image_owners()?, u64::try_from(count)?);
        assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
        assert_eq!(wgc::EVENT_CLOSE_ERROR.load(Ordering::Acquire), 0);
        assert_eq!(DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire), drop_errors);
        writeln!(finals, "{cycle}\t{count}\t{count}\t0\t0")?;
        finals.flush()?;
    }
    drop(owned);
    for (name, image) in &held {
        queries::save(&directory, &format!("{name}-held"), image.as_ref())?;
    }
    drop(held);
    assert_eq!(adapter.image_owners()?, 0);
    println!(
        "native Rust FramePool empty_client cycles=20 calls=60 real_zero=20 recovery=20 retained=40 owners_released=1 completion_states=0 restore_barriers={} passed",
        if prepare_restore { 20 } else { 0 }
    );
    Ok(())
}
