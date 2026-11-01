// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Controlled Size outcomes on real native items, original partial-state parity.
use super::*;
use crate::{
    backends::windows::parity_adapter::{Adapter, Image},
    platform::windows::wgc::size_probe,
};
use std::{error::Error as StdError, fs::OpenOptions, io::Write};

fn slots(capture: &FramePoolCapture<Image>) -> u32 {
    u32::from(capture.device.is_some())
        | (u32::from(capture.context.is_some()) << 1)
        | (u32::from(capture.swap_chain.is_some()) << 2)
        | (u32::from(capture.readable.is_some()) << 3)
        | (u32::from(capture.item.is_some()) << 4)
        | (u32::from(capture.pool.is_some()) << 5)
        | (u32::from(capture.session.is_some()) << 6)
}
#[test]
#[ignore = "explicit controlled original/Rust Size failures with actual native partial states"]
fn preserves_original_size_query_catches_partial_states_and_recovery()
-> Result<(), Box<dyn StdError>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_FRAMEPOOL_SIZE_ROOT").ok_or("missing Size root")?,
    );
    let directory = root.join("rust");
    std::fs::create_dir(&directory)?;
    let open = |name| {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(name))
    };
    let mut rows = open("size.tsv")?;
    let mut queries = open("queries.tsv")?;
    let mut states = open("rust-state.tsv")?;
    let mut finals = open("final.tsv")?;
    writeln!(
        rows,
        "cycle\tcase\tphase\toutcome\terror\tsize_calls\tbefore_slots\tafter_slots\tlast_width\tlast_height\tcache_preserved\tcache_independent\tmarker\tcaller_thread\tfixture_thread"
    )?;
    writeln!(
        queries,
        "cycle\tphase\tordinal\tnative_hresult\tnative_width\tnative_height\treturned_hresult\treturned_width\treturned_height\tcaller_thread\titem"
    )?;
    writeln!(
        states,
        "cycle\tphase\terror_code\tcopied\tconversions\tclones\towners\tcompletion_live\tclose_errors"
    )?;
    writeln!(finals, "cycle\theld\towners\tcompletion_live\tclose_errors")?;
    let mut apartment = host::APTTYPE::default();
    let mut qualifier = host::APTTYPEQUALIFIER::default();
    // SAFETY: Fresh caller owns only its successful initialization reference.
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
    let after = [127, 23, 7, 119, 7, 23, 119, 23, 7, 7];
    for cycle in 0..20_u32 {
        let mode = cycle % 10;
        let mut fixture = adapter.render_fixture(96, 64)?;
        let geometry = fixture.geometry([1, i64::from(0x9000_0000_u32), 100, 100, 96, 64])?;
        assert_eq!(&geometry[14..16], &[96, 64]);
        fixture.start_inside_desktop_updates()?;
        let threads = fixture.thread_info()?;
        let mut capture: FramePoolCapture<Image> = FramePoolCapture::new(fixture.hwnd());
        let mut converter = adapter.converter(0);
        for phase in ["warm", "probe", "recovery"] {
            let probe = phase == "probe";
            if probe && ((1..=3).contains(&mode) || mode >= 8) {
                assert!(capture.uninit().cleanup.iter().all(Option::is_none));
            }
            let before_slots = slots(&capture);
            let before = capture
                .cached
                .as_ref()
                .map(|image| (image.address(), image.bytes().to_vec()));
            let guard = size_probe::Guard::begin(if probe { mode } else { 0 });
            let attempt = capture.capture(&mut converter);
            let trace = guard.trace();
            drop(guard);
            let name = format!("{cycle}-{phase}");
            queries::save_pair_raw(
                &directory,
                &name,
                attempt.image.as_ref().ok(),
                capture.cached.as_ref(),
            )?;
            assert!(
                !attempt.evidence.replayed && attempt.evidence.cleanup.iter().all(Option::is_none)
            );
            let failure = probe && mode != 0;
            let exception = failure && [1, 3, 6, 7].contains(&mode);
            let expected = if exception { 2 } else { u32::from(!failure) };
            let mut error_code = 0;
            let image = if failure {
                match &attempt.image {
                    Err(Error::WinRtException("GraphicsCaptureItem.Size", code)) if exception => {
                        assert_eq!(code.0, size_probe::INJECTED);
                        error_code = code.0;
                    }
                    Err(Error::Native(operation, code)) if [2, 4, 5, 9].contains(&mode) => {
                        assert_eq!(
                            *operation,
                            if mode == 4 {
                                "GraphicsCaptureItem.Size"
                            } else if mode == 9 {
                                "Direct3D11CaptureFramePool.Create"
                            } else {
                                "GraphicsCaptureItem.Size for pool"
                            }
                        );
                        assert_eq!(
                            code.0,
                            if mode == 9 {
                                0x8007_0057_u32 as i32
                            } else {
                                size_probe::INJECTED
                            }
                        );
                        error_code = code.0;
                    }
                    Err(Error::InvalidItemSize {
                        width: 0,
                        height: 64,
                    }) if mode == 8 => {}
                    Err(error) => panic!("unexpected Size boundary error: {error:?}"),
                    Ok(_) => panic!("Size fault unexpectedly returned an image"),
                }
                assert_eq!(attempt.evidence.target_copy_bytes, 0);
                assert_eq!(attempt.evidence.polls, 0);
                None
            } else {
                Some(
                    attempt
                        .image
                        .map_err(|error| format!("Size cycle={cycle} {phase}: {error:?}"))?,
                )
            };
            assert_eq!(
                slots(&capture),
                if probe { after[mode as usize] } else { 127 }
            );
            assert_eq!(capture.last_size, if failure { (0, 0) } else { (96, 64) });
            let cache = capture.cached.as_ref().ok_or("Size warm cache lost")?;
            queries::known(cache);
            if failure {
                let (address, bytes) = before.as_ref().ok_or("missing warm cache")?;
                assert_eq!(cache.address(), *address);
                assert_eq!(cache.bytes(), bytes);
            }
            if let Some(image) = &image {
                assert_ne!(image.address(), cache.address());
                assert_eq!(image.bytes(), cache.bytes());
            }
            let marker = image.as_ref().map_or(-1, queries::known);
            queries::validate_saved(image.as_ref());
            queries::validate_saved(Some(cache));
            writeln!(
                rows,
                "{cycle}\t{mode}\t{phase}\t{expected}\t{}\t{}\t{before_slots}\t{}\t{}\t{}\t{}\t1\t{marker}\t{}\t{}",
                if exception { size_probe::INJECTED } else { 0 },
                trace.calls,
                slots(&capture),
                capture.last_size.0,
                capture.last_size.1,
                u8::from(failure),
                threads[1],
                threads[0]
            )?;
            for (i, q) in trace.queries[..trace.calls].iter().enumerate() {
                assert_eq!(
                    (q.native_code, q.width, q.height, q.thread),
                    (0, 96, 64, u32::try_from(threads[1])?)
                );
                assert_ne!(q.item, 0);
                writeln!(
                    queries,
                    "{cycle}\t{phase}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    i + 1,
                    q.native_code,
                    q.width,
                    q.height,
                    q.returned_code,
                    q.returned_width,
                    q.returned_height,
                    q.thread,
                    q.item
                )?;
            }
            held.push((name, image));
            let count = held.iter().filter(|(_, image)| image.is_some()).count();
            assert_eq!(adapter.image_owners()?, u64::try_from(count + 1)?);
            assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
            writeln!(
                states,
                "{cycle}\t{phase}\t{error_code}\t{}\t{}\t{}\t{}\t0\t0",
                attempt.evidence.target_copy_bytes,
                converter.conversions,
                converter.clones,
                count + 1
            )?;
            rows.flush()?;
            queries.flush()?;
            states.flush()?;
        }
        assert!(capture.uninit().cleanup.iter().all(Option::is_none));
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
        queries::save(&directory, &format!("{name}-held"), image.as_ref())?;
    }
    drop(held);
    assert_eq!(adapter.image_owners()?, 0);
    println!(
        "native Rust FramePool size_boundary cycles=20 calls=60 rejected=18 recovery=20 retained=42 owners_released=1 completion_states=0 passed"
    );
    Ok(())
}
