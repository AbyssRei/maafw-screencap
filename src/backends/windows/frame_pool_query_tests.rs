// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real mapped frames with scoped query faults, compared to the original class.
use super::*;
use crate::backends::windows::parity_adapter::{Adapter, Image};
use std::{error::Error as StdError, fs::OpenOptions, io::Write, path::Path};

pub(super) fn known(image: &Image) -> i32 {
    assert_eq!(image.dimensions(), (96, 64));
    assert_eq!(image.bytes().len(), 18432);
    let marker = &image.bytes()[..3];
    let phase = match marker {
        [0x4e, 0x5f, 0x60] => 0,
        [0x1a, 0x2b, 0x3c] => 1,
        _ => panic!("unexpected FramePool query marker: {marker:?}"),
    };
    for y in 0..64 {
        for x in 0..96 {
            let expected = if x < 16 && y < 16 {
                marker
            } else if x < 32 {
                &[0x1f, 0x53, 0xa7]
            } else if y < 32 {
                &[0xb9, 0x4d, 0x27]
            } else {
                &[0x3b, 0xe2, 0x81]
            };
            let offset = (y * 96 + x) * 3;
            assert_eq!(&image.bytes()[offset..offset + 3], expected);
        }
    }
    phase
}
pub(super) fn save(
    root: &Path,
    name: &str,
    image: Option<&Image>,
) -> Result<(), Box<dyn StdError>> {
    let mut layout = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(format!("{name}.txt")))?;
    let mut pixels = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(format!("{name}.bgr")))?;
    if let Some(image) = image {
        known(image);
        assert_eq!(image.stride(), 288);
        writeln!(layout, "96\t64\t288\t18432")?;
        pixels.write_all(image.bytes())?;
    } else {
        writeln!(layout, "0\t0\t0\t0")?;
    }
    Ok(())
}

#[test]
#[ignore = "explicit original/Rust native FramePool query matrix and records required"]
fn matches_original_mapped_query_failures_fallback_and_roi_clamps() -> Result<(), Box<dyn StdError>>
{
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_FRAMEPOOL_QUERY_ROOT").ok_or("missing query root")?,
    );
    let directory = root.join("rust");
    std::fs::create_dir(&directory)?;
    let mut rows = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("queries.tsv"))?;
    writeln!(
        rows,
        "cycle\tcase\tphase\tstatus\torder\twin32\tdwm\tmarker\tcaller_thread\tfixture_thread\tticks"
    )?;
    let mut states = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("rust-state.tsv"))?;
    writeln!(
        states,
        "cycle\tphase\tpitch\tcopied\tconversions\tclones\tcache_same\tnative_slots\towners\tcompletion_live\tclose_errors"
    )?;
    let mut finals = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("final.tsv"))?;
    writeln!(finals, "cycle\theld\towners\tcompletion_live\tclose_errors")?;
    let mut apartment = host::APTTYPE::default();
    let mut qualifier = host::APTTYPEQUALIFIER::default();
    // SAFETY: Observe this fresh test thread, then own one successful MTA ref.
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
        let mode = cycle % 10;
        let mut fixture = adapter.render_fixture(96, 64)?;
        // Exact borderless scene makes every injected clamp have a known ROI.
        let geometry = fixture.geometry([1, i64::from(0x9000_0000_u32), 100, 100, 96, 64])?;
        assert_eq!(&geometry[14..16], &[96, 64]);
        fixture.start_inside_desktop_updates()?;
        let owner = fixture.thread_info()?;
        let mut capture: FramePoolCapture<Image> = FramePoolCapture::new(fixture.hwnd());
        let mut converter = adapter.converter(0);
        let mut successes = 0;
        for phase in ["warm", "probe", "recovery"] {
            let current = if phase == "probe" { mode } else { 0 };
            let failure = matches!(current, 1 | 2 | 4);
            let before = capture
                .cached
                .as_ref()
                .map(|image| (image.address(), image.bytes().to_vec()));
            let guard = query_probe::Guard::begin(current);
            let attempt = capture.capture(&mut converter); // No capture retry.
            let trace = guard.trace();
            drop(guard);
            let order = match current {
                1 => 1,
                2 => 12,
                3 | 4 => 1234,
                _ => 123,
            };
            assert_eq!((trace.case, trace.order), (current, order));
            assert!(!attempt.evidence.replayed);
            assert!(attempt.evidence.polls > 0 && attempt.evidence.mapped_pitch >= 384);
            assert!(attempt.evidence.cleanup.iter().all(Option::is_none));
            assert_eq!(
                attempt.evidence.frame_bounds_fallback.map(|code| code.0),
                if matches!(current, 3 | 4) {
                    Some(trace.dwm)
                } else {
                    None
                }
            );
            assert_eq!(trace.win32 != 0, failure);
            assert_eq!(trace.dwm < 0, matches!(current, 3 | 4));
            let image = if failure {
                let operation = match current {
                    1 => "GetClientRect",
                    2 => "ClientToScreen",
                    _ => "GetWindowRect",
                };
                assert!(
                    matches!(attempt.image, Err(Error::Win32(op, code)) if op == operation && code == trace.win32)
                );
                let (address, bytes) = before.as_ref().ok_or("missing warm cache")?;
                assert_eq!(capture.cached.as_ref().unwrap().address(), *address);
                assert_eq!(capture.cached.as_ref().unwrap().bytes(), bytes);
                None
            } else {
                successes += 1;
                Some(
                    attempt
                        .image
                        .map_err(|e| format!("query cycle={cycle} phase={phase}: {e:?}"))?,
                )
            };
            assert_eq!(
                attempt.evidence.target_copy_bytes,
                if failure { 0 } else { 24576 }
            );
            assert_eq!(
                (converter.conversions, converter.clones),
                (successes, successes)
            );
            assert!(
                capture.device.is_some()
                    && capture.context.is_some()
                    && capture.swap_chain.is_some()
                    && capture.readable.is_some()
                    && capture.item.is_some()
                    && capture.pool.is_some()
                    && capture.session.is_some()
            );
            let marker = image.as_ref().map_or(-1, known);
            let updates = fixture.desktop_update_info()?;
            assert!(updates[0] != 0 && updates[1] != 0 && updates[2] == 0);
            let name = format!("{cycle}-{phase}");
            save(&directory, &name, image.as_ref())?;
            writeln!(
                rows,
                "{cycle}\t{mode}\t{phase}\t{}\t{}\t{}\t{}\t{marker}\t{}\t{}\t{}",
                u8::from(image.is_some()),
                trace.order,
                trace.win32,
                trace.dwm,
                owner[1],
                owner[0],
                updates[1]
            )?;
            held.push((name, image));
            let count = held.iter().filter(|(_, image)| image.is_some()).count();
            assert_eq!(adapter.image_owners()?, u64::try_from(count + 1)?);
            assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
            assert_eq!(DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire), drop_errors);
            writeln!(
                states,
                "{cycle}\t{phase}\t{}\t{}\t{successes}\t{successes}\t{}\t1\t{}\t0\t0",
                attempt.evidence.mapped_pitch,
                attempt.evidence.target_copy_bytes,
                u8::from(failure),
                count + 1
            )?;
            rows.flush()?;
            states.flush()?;
        }
        let closed = capture.uninit();
        assert!(closed.cleanup.iter().all(Option::is_none));
        drop(capture);
        fixture.close()?;
        let count = held.iter().filter(|(_, image)| image.is_some()).count();
        assert_eq!(adapter.image_owners()?, u64::try_from(count)?);
        assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
        assert_eq!(wgc::EVENT_CLOSE_ERROR.load(Ordering::Acquire), 0);
        assert_eq!(DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire), drop_errors);
        writeln!(finals, "{cycle}\t{count}\t{count}\t0\t0")?;
        finals.flush()?;
    }
    drop(owned);
    for (name, image) in &held {
        save(&directory, &format!("{name}-held"), image.as_ref())?;
    }
    drop(held);
    assert_eq!(adapter.image_owners()?, 0);
    println!(
        "native Rust FramePool queries cycles=20 cases=10 calls=60 images=54 rejected=6 recovery=20 retained=54 owners_released=1 completion_states=0 passed"
    );
    Ok(())
}
