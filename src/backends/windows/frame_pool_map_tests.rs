// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real native map failure and conversion error with exact Unmap observation.
use super::*;
use crate::{
    backends::windows::parity_adapter::{Adapter, Image},
    platform::windows::wgc::map_probe,
};
use std::{error::Error as StdError, fs::OpenOptions, io::Write};
struct TracedConverter<'a>(crate::backends::windows::parity_adapter::Converter<'a>);
impl BgraConverter for TracedConverter<'_> {
    type Image = Image;
    type Error = i32;
    fn bgra_to_bgr(&mut self, view: BgraView<'_>) -> Result<Image, i32> {
        if map_probe::conversion() {
            return Err(2);
        }
        self.0.bgra_to_bgr(view)
    }
}
impl Converter for TracedConverter<'_> {
    fn clone_image(&mut self, image: &Image) -> Result<Image, i32> {
        let result = self.0.clone_image(image)?;
        map_probe::cache_clone();
        Ok(result)
    }
}
#[test]
#[ignore = "explicit original/Rust real D3D Map failure and Unmap/conversion records required"]
fn preserves_warm_cache_and_unmaps_exactly_once_across_native_map_and_conversion_failures()
-> Result<(), Box<dyn StdError>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_FRAMEPOOL_MAP_ROOT").ok_or("missing map-boundary root")?,
    );
    let directory = root.join("rust");
    std::fs::create_dir(&directory)?;
    let mut rows = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("map.tsv"))?;
    writeln!(
        rows,
        "cycle\tcase\tphase\toutcome\torder\tmaps\tunmaps\tconversions\tcache_clones\thresult\tsubresource\tpitch\tmapped\tviolation\tquery_order\tcache_preserved\tcache_independent\tmarker\tcaller_thread\tfixture_thread\tcontext\tresource"
    )?;
    let mut states = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("rust-state.tsv"))?;
    writeln!(
        states,
        "cycle\tphase\tcopied\tconversions\tclones\towners\tcompletion_live\tclose_errors"
    )?;
    let mut finals = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("final.tsv"))?;
    writeln!(finals, "cycle\theld\towners\tcompletion_live\tclose_errors")?;
    let mut apartment = host::APTTYPE::default();
    let mut qualifier = host::APTTYPEQUALIFIER::default();
    // SAFETY: This fresh caller owns only its successful MTA reference.
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
        let mode = cycle % 3;
        let mut fixture = adapter.render_fixture(96, 64)?;
        let geometry = fixture.geometry([1, i64::from(0x9000_0000_u32), 100, 100, 96, 64])?;
        assert_eq!(&geometry[14..16], &[96, 64]);
        fixture.start_inside_desktop_updates()?;
        let threads = fixture.thread_info()?;
        let mut capture: FramePoolCapture<Image> = FramePoolCapture::new(fixture.hwnd());
        let mut converter = TracedConverter(adapter.converter(0));
        let mut context = 0;
        let mut resource = 0;
        let mut warm_address = 0;
        for phase in ["warm", "probe", "recovery"] {
            let fault = if phase == "probe" { mode } else { 0 };
            let before = capture
                .cached
                .as_ref()
                .map(|image| (image.address(), image.bytes().to_vec()));
            let guard = map_probe::Guard::begin(fault);
            let queries_guard = query_probe::Guard::begin(0);
            let attempt = capture.capture(&mut converter);
            let query = queries_guard.trace();
            let trace = guard.trace();
            drop(queries_guard);
            drop(guard);
            let name = format!("{cycle}-{phase}");
            queries::save_pair_raw(
                &directory,
                &name,
                attempt.image.as_ref().ok(),
                capture.cached.as_ref(),
            )?;
            assert_eq!(
                trace.order,
                match fault {
                    1 => 12,
                    2 => 1235,
                    _ => 12345,
                }
            );
            assert_eq!(
                (
                    trace.maps,
                    trace.unmaps,
                    trace.conversions,
                    trace.cache_clones
                ),
                (
                    1,
                    u32::from(fault != 1),
                    u32::from(fault != 1),
                    u32::from(fault == 0)
                )
            );
            assert!(!trace.mapped && !trace.violation);
            assert_eq!(u64::from(trace.thread), threads[1]);
            assert_eq!(trace.subresource, if fault == 1 { u32::MAX } else { 0 });
            assert_eq!(query.order, if fault == 1 { 0 } else { 123 });
            assert_eq!((query.win32, query.dwm), (0, 0));
            assert!(
                !attempt.evidence.replayed && attempt.evidence.cleanup.iter().all(Option::is_none)
            );
            if context == 0 {
                context = trace.context;
                resource = trace.resource;
            }
            assert_ne!(context, 0);
            assert_ne!(resource, 0);
            assert_eq!((trace.context, trace.resource), (context, resource));
            let outcome = if fault == 1 {
                0
            } else if fault == 2 {
                2
            } else {
                1
            };
            let image = match fault {
                1 => {
                    assert!(trace.map_hresult < 0);
                    assert!(
                        matches!(attempt.image,Err(Error::Native("Map",code)) if code.0==trace.map_hresult)
                    );
                    assert_eq!(trace.pitch, 0);
                    None
                }
                2 => {
                    assert!(matches!(attempt.image, Err(Error::Conversion(2))));
                    assert!(trace.map_hresult >= 0 && trace.pitch >= 384);
                    None
                }
                _ => {
                    assert!(trace.map_hresult >= 0 && trace.pitch >= 384);
                    Some(
                        attempt
                            .image
                            .map_err(|e| format!("map cycle={cycle} phase={phase}: {e:?}"))?,
                    )
                }
            };
            assert_eq!(
                attempt.evidence.target_copy_bytes,
                if fault == 1 { 0 } else { 24576 }
            );
            let cache = capture
                .cached
                .as_ref()
                .ok_or("map boundary lost warm cache")?;
            queries::known(cache);
            if fault != 0 {
                let (address, bytes) = before.as_ref().ok_or("missing pre-error cache")?;
                assert_eq!(cache.address(), *address);
                assert_eq!(cache.bytes(), bytes);
                assert_ne!(cache.address(), warm_address);
            } else {
                let image = image.as_ref().unwrap();
                assert_ne!(cache.address(), image.address());
                assert_eq!(cache.bytes(), image.bytes());
                if let Some((address, _)) = &before {
                    assert_ne!(cache.address(), *address);
                }
                if phase == "warm" {
                    warm_address = image.address();
                }
            }
            let marker = image.as_ref().map_or(-1, queries::known);
            queries::validate_saved(image.as_ref());
            queries::validate_saved(Some(cache));
            held.push((name, image));
            let count = held.iter().filter(|(_, image)| image.is_some()).count();
            assert_eq!(adapter.image_owners()?, u64::try_from(count + 1)?);
            assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
            writeln!(
                rows,
                "{cycle}\t{mode}\t{phase}\t{outcome}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t0\t0\t{}\t{}\t1\t{marker}\t{}\t{}\t{context}\t{resource}",
                trace.order,
                trace.maps,
                trace.unmaps,
                trace.conversions,
                trace.cache_clones,
                trace.map_hresult,
                trace.subresource,
                trace.pitch,
                query.order,
                u8::from(fault != 0),
                threads[1],
                threads[0]
            )?;
            writeln!(
                states,
                "{cycle}\t{phase}\t{}\t{}\t{}\t{}\t0\t0",
                attempt.evidence.target_copy_bytes,
                converter.0.conversions,
                converter.0.clones,
                count + 1
            )?;
            rows.flush()?;
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
        "native Rust FramePool map_boundary cycles=20 calls=60 map_failures=7 conversion_errors=6 recovery=20 retained=47 owners_released=1 completion_states=0 passed"
    );
    Ok(())
}
