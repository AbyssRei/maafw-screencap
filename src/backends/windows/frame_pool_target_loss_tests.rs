// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Repeated native target destruction against the separate original program.
use super::*;
use crate::backends::windows::parity_adapter::{Adapter, Image};
use std::{
    error::Error as StdError, fs::OpenOptions, io::Write, path::Path, sync::atomic::Ordering,
};

fn save(root: &Path, name: &str, image: &Image) -> Result<(), Box<dyn StdError>> {
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
    let mut pixels = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(format!("{name}.bgr")))?;
    pixels.write_all(image.bytes())?;
    Ok(())
}
fn known(image: &Image) {
    assert_eq!(image.dimensions(), (96, 64));
    assert_eq!(image.bytes().len(), 96 * 64 * 3);
    for y in 8..56 {
        for x in 8..88 {
            let expected = if x < 32 {
                [0x1f, 0x53, 0xa7]
            } else if y < 32 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            let offset = (y * 96 + x) * 3;
            assert_eq!(image.bytes()[offset..offset + 3], expected);
        }
    }
}

#[test]
#[ignore = "explicit native source/Rust repeated target destruction and original records required"]
fn matches_original_repeated_target_loss_without_replaying_cached_pixels()
-> Result<(), Box<dyn StdError>> {
    let root = std::env::var_os("MFS_FRAMEPOOL_TARGET_LOSS_ROOT")
        .ok_or("missing target-loss evidence root")?;
    let root = std::path::PathBuf::from(root);
    let directory = root.join("rust");
    std::fs::create_dir(&directory)?;
    let mut rows = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("target-loss.tsv"))?;
    writeln!(
        rows,
        "cycle\tinvalid_window\trejected_calls\twidth\theight\tfixture_closed\tsource_destroyed\tcaller_thread\tfixture_thread\telapsed_ms"
    )?;
    let mut states = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("rust-state.tsv"))?;
    writeln!(
        states,
        "cycle\tinvalid_calls\tpolls\tcopied\treplayed\tcache_same\tnative_slots\tconversions\tclones\towners_after_capture\towners_after_drop\tcompletion_live\tclose_errors"
    )?;
    let mut apartment = host::APTTYPE::default();
    let mut qualifier = host::APTTYPEQUALIFIER::default();
    // SAFETY: Read-only observation of this exact fresh test thread.
    assert_eq!(
        unsafe { host::CoGetApartmentType(&mut apartment, &mut qualifier) },
        host::CO_E_NOTINITIALIZED
    );
    // SAFETY: Own only the successful reference on this fresh MTA caller.
    unsafe { host::RoInitialize(host::RO_INIT_MULTITHREADED).ok()? };
    let owned = HostInitialization(true);
    let adapter = Adapter::load()?;
    assert_eq!(adapter.image_owners()?, 0);
    let drop_errors = DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire);
    let mut held = Vec::new();
    held.try_reserve_exact(20)?;
    for cycle in 0..20_u64 {
        let mut fixture = adapter.render_fixture(96, 64)?;
        let owner = fixture.thread_info()?;
        assert_ne!(owner[0], 0);
        assert_eq!(owner[0], owner[2]);
        assert_ne!(owner[0], owner[1]);
        let mut capture: FramePoolCapture<Image> = FramePoolCapture::new(fixture.hwnd());
        let mut converter = adapter.converter(0);
        let attempt = capture.capture(&mut converter);
        assert!(attempt.evidence.cleanup.iter().all(Option::is_none));
        assert!(!attempt.evidence.replayed);
        assert_eq!(attempt.evidence.target_copy_bytes, 96 * 64 * 4);
        let first = attempt
            .image
            .map_err(|e| format!("target-loss initial cycle {cycle}: {e:?}"))?;
        known(&first);
        assert_eq!(
            first.bytes(),
            std::fs::read(root.join("source").join(format!("{cycle}.bgr")))?
        );
        save(&directory, &cycle.to_string(), &first)?;
        let cache = capture.cached.as_ref().unwrap().address();
        assert_ne!(first.address(), cache);
        assert_eq!(adapter.image_owners()?, cycle + 2);
        fixture.close()?; // Native owner-thread destruction and completed join.
        // SAFETY: IsWindow observes this nonowning, now-destroyed scalar HWND.
        assert!(!unsafe { IsWindow(Some(capture.window)) }.as_bool());
        let began = std::time::Instant::now();
        for _ in 0..3 {
            let lost = capture.capture(&mut converter);
            assert!(matches!(lost.image, Err(Error::InvalidWindow)));
            assert_eq!(
                (lost.evidence.polls, lost.evidence.target_copy_bytes),
                (0, 0)
            );
            assert!(!lost.evidence.replayed);
            assert!(lost.evidence.cleanup.iter().all(Option::is_none));
            assert!(capture.pool.is_some() && capture.session.is_some() && capture.item.is_some());
            assert_eq!(capture.cached.as_ref().unwrap().address(), cache);
            assert_eq!(capture.cached.as_ref().unwrap().bytes(), first.bytes());
        }
        let elapsed = began.elapsed().as_millis();
        assert_eq!((converter.conversions, converter.clones), (1, 1));
        let closed = capture.uninit();
        assert!(closed.cleanup.iter().all(Option::is_none));
        assert!(capture.pool.is_none() && capture.session.is_none() && capture.item.is_none());
        drop(capture);
        assert_eq!(adapter.image_owners()?, cycle + 1);
        assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
        assert_eq!(wgc::EVENT_CLOSE_ERROR.load(Ordering::Acquire), 0);
        assert_eq!(DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire), drop_errors);
        writeln!(
            rows,
            "{cycle}\t1\t3\t96\t64\t1\t1\t{}\t{}\t{elapsed}",
            owner[1], owner[0]
        )?;
        writeln!(
            states,
            "{cycle}\t3\t0\t0\t0\t1\t1\t1\t1\t{}\t{}\t0\t0",
            cycle + 2,
            cycle + 1
        )?;
        rows.flush()?;
        states.flush()?;
        held.push(first);
    }
    drop(owned);
    for (cycle, image) in held.iter().enumerate() {
        known(image);
        save(&directory, &format!("{cycle}-held"), image)?;
        assert_eq!(
            image.bytes(),
            std::fs::read(directory.join(format!("{cycle}.bgr")))?
        );
    }
    drop(held);
    assert_eq!(adapter.image_owners()?, 0);
    println!(
        "native Rust FramePool target_loss cycles=20 captures=20 rejected_calls=60 retained=20 fixture_joins=20 image_owners_released=1 completion_states=0 passed"
    );
    Ok(())
}
