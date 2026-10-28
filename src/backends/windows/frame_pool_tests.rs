// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;

#[cfg(feature = "prototype-windows-com")]
#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
mod host {
    include!(concat!(env!("OUT_DIR"), "/windows_com_prototype.rs"));
}

#[cfg(feature = "prototype-windows-com")]
struct HostInitialization(bool);
#[cfg(feature = "prototype-windows-com")]
impl Drop for HostInitialization {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: Own one successful caller-thread reference, including S_FALSE.
            unsafe {
                host::RoUninitialize();
            }
            self.0 = false;
        }
    }
}

// Harness-only current-thread queue. Capture still runs synchronously without
// pumping. Only shutdown dispatches messages, after every capture owner closes.
#[cfg(feature = "prototype-windows-com")]
struct HostDispatcher(Option<host::DispatcherQueueController>);
#[cfg(feature = "prototype-windows-com")]
impl HostDispatcher {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // SAFETY: A fresh test thread owns its successful MTA initialization;
        // current-thread queue creation does not request an apartment change.
        let controller = unsafe {
            host::CreateDispatcherQueueController(host::DispatcherQueueOptions {
                dwSize: u32::try_from(size_of::<host::DispatcherQueueOptions>())?,
                threadType: host::DQTYPE_THREAD_CURRENT,
                apartmentType: host::DQTAT_COM_NONE,
            })?
        };
        Ok(Self(Some(controller.cast()?)))
    }

    fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(controller) = self.0.as_ref() else {
            return Ok(());
        };
        // Any native shutdown failure aborts this test process; do not reissue
        // shutdown, unload an active queue, or turn an observation timeout into
        // permission to abandon the caller's owned native thread.
        let result = (|| -> Result<bool, Box<dyn std::error::Error>> {
            let operation = controller.ShutdownQueueAsync()?;
            let began = Instant::now();
            let mut over_budget = false;
            while operation.Status()? == windows_future::AsyncStatus::Started {
                let mut message = host::MSG::default();
                // SAFETY: Read and dispatch only this harness thread's queue.
                while unsafe {
                    host::PeekMessageW(&mut message, None, 0, 0, u32::try_from(host::PM_REMOVE)?)
                }
                .as_bool()
                {
                    if message.message == u32::try_from(host::WM_QUIT)? {
                        return Err("unexpected WM_QUIT during dispatcher shutdown".into());
                    }
                    unsafe {
                        let _ = host::TranslateMessage(&message);
                        host::DispatchMessageW(&message);
                    }
                }
                over_budget |= began.elapsed() > Duration::from_secs(5);
                std::thread::sleep(Duration::from_millis(1));
            }
            operation.GetResults()?;
            Ok(over_budget)
        })();
        match result {
            Ok(over_budget) => {
                self.0.take();
                println!("Rust dispatcher_shutdown=completed over_budget={over_budget}");
                if over_budget {
                    return Err("dispatcher shutdown exceeded observation budget".into());
                }
                Ok(())
            }
            Err(error) => {
                eprintln!("native dispatcher shutdown failed: {error}");
                std::process::abort();
            }
        }
    }
}
#[cfg(feature = "prototype-windows-com")]
impl Drop for HostDispatcher {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            // Queue completion has already been established for this branch;
            // retain a failed test result even on an earlier assertion unwind.
            eprintln!("dispatcher cleanup exceeded observation budget: {error}");
            std::process::abort();
        }
    }
}

#[test]
fn preserves_source_edge_clamping_and_rejects_geometry_overflow() {
    let client = RECT {
        left: 0,
        top: 0,
        right: 37,
        bottom: 23,
    };
    let bounds = RECT {
        left: 100,
        top: 200,
        right: 164,
        bottom: 231,
    };
    let roi = client_roi::<()>(client, POINT { x: 155, y: 224 }, bounds, 64, 31).unwrap();
    assert_eq!(
        roi,
        Roi {
            left: 27,
            top: 8,
            width: 37,
            height: 23
        }
    );
    let roi = client_roi::<()>(client, POINT { x: 90, y: 180 }, bounds, 19, 11).unwrap();
    assert_eq!(
        roi,
        Roi {
            left: 0,
            top: 0,
            width: 19,
            height: 11
        }
    );
    assert!(matches!(
        client_roi::<()>(client, POINT { x: i32::MIN, y: 0 }, bounds, 64, 31),
        Err(Error::Overflow)
    ));
    assert_eq!(
        client_roi::<()>(RECT::default(), POINT::default(), bounds, 64, 31).unwrap(),
        Roi {
            left: 0,
            top: 0,
            width: 0,
            height: 0
        }
    );
}

#[test]
fn copies_only_target_pixels_from_padded_rows_into_independent_storage() {
    let mut bytes = [0xce_u8; 3 * 32];
    for y in 0..3 {
        for x in 0..5 {
            let start = y * 32 + x * 4;
            bytes[start..start + 4].copy_from_slice(&[
                u8::try_from(x).unwrap(),
                u8::try_from(y).unwrap(),
                0xa7,
                0xff,
            ]);
        }
    }
    let desc = D3D11_TEXTURE2D_DESC {
        Width: 5,
        Height: 3,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        MipLevels: 1,
        ArraySize: 1,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        ..Default::default()
    };
    let mapped = D3D11_MAPPED_SUBRESOURCE {
        pData: bytes.as_mut_ptr().cast(),
        RowPitch: 32,
        DepthPitch: 96,
    };
    // SAFETY: This fully initialized owned allocation supplies every described
    // row through this synchronous copy; no native buffer is fabricated/retained.
    let pixels = unsafe {
        copy_target::<()>(
            &mapped,
            desc,
            Roi {
                left: 2,
                top: 1,
                width: 2,
                height: 2,
            },
        )
    }
    .unwrap();
    assert_eq!(
        pixels,
        [
            2, 1, 0xa7, 0xff, 3, 1, 0xa7, 0xff, 2, 2, 0xa7, 0xff, 3, 2, 0xa7, 0xff
        ]
    );
    bytes.fill(0);
    assert_eq!(pixels[0..4], [2, 1, 0xa7, 0xff]);
    // SAFETY: Same live storage; invalid pitch must be rejected before any read.
    let invalid = D3D11_MAPPED_SUBRESOURCE {
        RowPitch: 19,
        ..mapped
    };
    assert!(matches!(
        unsafe {
            copy_target::<()>(
                &invalid,
                desc,
                Roi {
                    left: 0,
                    top: 0,
                    width: 1,
                    height: 1,
                },
            )
        },
        Err(Error::InvalidLayout)
    ));
}

#[cfg(feature = "prototype-windows-com")]
#[test]
#[ignore = "explicit native FramePool migration; original pixels and native conversion adapter required"]
fn captures_original_framepool_pixels_and_retains_result_and_cache_clone_owners()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::parity_adapter::{Adapter, Image};
    use std::sync::atomic::Ordering;
    let mode = std::env::var("MFS_FRAMEPOOL_HOST_MODE").unwrap_or_else(|_| "uninitialized".into());
    let mut apartment = host::APTTYPE::default();
    let mut qualifier = host::APTTYPEQUALIFIER::default();
    // SAFETY: Read-only caller-thread apartment/DPI observation, exact outputs.
    let before = unsafe { host::CoGetApartmentType(&mut apartment, &mut qualifier) };
    assert_eq!(
        before,
        host::CO_E_NOTINITIALIZED,
        "each host context requires a fresh test process"
    );
    let dpi = unsafe { host::GetThreadDpiAwarenessContext() };
    let mut owned = HostInitialization(false);
    match mode.as_str() {
        "uninitialized" => {}
        "mta" | "mta-dispatcher" | "sta" => {
            let kind = if mode != "sta" {
                host::RO_INIT_MULTITHREADED
            } else {
                host::RO_INIT_SINGLETHREADED
            };
            // SAFETY: This test owns a fresh caller thread; no host mode change.
            unsafe {
                host::RoInitialize(kind).ok()?;
            }
            owned.0 = true;
        }
        _ => return Err("supported contexts: uninitialized, mta, mta-dispatcher, sta".into()),
    }
    let mut dispatcher = if mode == "mta-dispatcher" {
        Some(HostDispatcher::new()?)
    } else {
        None
    };
    let reference = std::env::var_os("MFS_FRAMEPOOL_REFERENCE_PIXELS")
        .ok_or("original framepool pixels required")?;
    let reference = std::path::PathBuf::from(reference);
    if !reference.is_absolute() {
        return Err("source pixels path must be absolute".into());
    }
    let expected = std::fs::read(reference)?;
    assert_eq!(expected.len(), 96 * 64 * 3);
    let adapter = Adapter::load()?;
    let mut fixture = adapter.render_fixture(96, 64)?;
    let mut capture: FramePoolCapture<Image> = FramePoolCapture::new(fixture.hwnd());
    let mut converter = adapter.converter(0);
    let first = capture.capture(&mut converter);
    println!("FramePool first: {:?}", first.evidence);
    assert!(first.evidence.cleanup.iter().all(Option::is_none));
    assert!(!first.evidence.replayed);
    let first = first
        .image
        .map_err(|error| format!("first capture {error:?}"))?;
    assert_eq!(first.dimensions(), (96, 64));
    assert_eq!(first.bytes(), expected);
    assert_eq!((converter.conversions, converter.clones), (1, 1));
    assert_ne!(first.address(), capture.cached.as_ref().unwrap().address());
    assert_eq!(first.native_owners(), 2);
    let second = capture.capture(&mut converter);
    println!("FramePool second: {:?}", second.evidence);
    assert!(second.evidence.cleanup.iter().all(Option::is_none));
    let replayed = second.evidence.replayed;
    let second = second
        .image
        .map_err(|error| format!("second capture {error:?}"))?;
    assert_eq!(second.bytes(), expected);
    assert_eq!(converter.conversions, if replayed { 1 } else { 2 });
    assert_eq!(converter.clones, 2);
    assert_ne!(second.address(), capture.cached.as_ref().unwrap().address());
    assert_eq!(second.native_owners(), 3);
    let closed = capture.uninit();
    assert!(closed.cleanup.iter().all(Option::is_none));
    assert!(
        capture.cached.is_some()
            && capture.device.is_some()
            && capture.context.is_some()
            && capture.swap_chain.is_some()
    );
    assert!(
        capture.pool.is_none()
            && capture.session.is_none()
            && capture.item.is_none()
            && capture.readable.is_none()
    );
    drop(capture);
    fixture.close()?;
    drop(adapter);
    assert_eq!(first.bytes(), expected);
    assert_eq!(second.bytes(), expected);
    assert_eq!(second.native_owners(), 2);
    drop(first);
    assert_eq!(second.native_owners(), 1);
    assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
    assert_eq!(wgc::EVENT_CLOSE_ERROR.load(Ordering::Acquire), 0);
    if let Some(dispatcher) = dispatcher.as_mut() {
        dispatcher.close()?;
    }
    // Only independent CPU images remain at caller uninitialization. Native
    // capture/session/frame/device/factory owners have all been released.
    drop(owned);
    let after = unsafe { host::CoGetApartmentType(&mut apartment, &mut qualifier) };
    println!("FramePool host={mode}, final apartment={after:?}/{apartment:?}/{qualifier:?}");
    if mode == "uninitialized" {
        assert!(after.is_ok());
        assert_eq!(
            (apartment, qualifier),
            (host::APTTYPE_MTA, host::APTTYPEQUALIFIER_IMPLICIT_MTA)
        );
    } else {
        assert_eq!(after, host::CO_E_NOTINITIALIZED);
    }
    assert!(
        unsafe { host::AreDpiAwarenessContextsEqual(dpi, host::GetThreadDpiAwarenessContext()) }
            .as_bool()
    );
    Ok(())
}

#[cfg(feature = "prototype-windows-com")]
#[test]
#[ignore = "explicit twenty native FramePool lifecycles; original image and native adapter required"]
fn retains_pixels_across_twenty_complete_framepool_lifecycles()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::parity_adapter::{Adapter, Image};
    let reference = std::env::var_os("MFS_FRAMEPOOL_REFERENCE_PIXELS")
        .ok_or("original framepool pixels required")?;
    let reference = std::path::PathBuf::from(reference);
    if !reference.is_absolute() {
        return Err("source pixels path must be absolute".into());
    }
    let expected = std::fs::read(reference)?;
    assert_eq!(expected.len(), 96 * 64 * 3);
    // SAFETY: Explicit successful initialization reference on this fresh test
    // thread. It is released only after all captures and dispatcher shutdown.
    unsafe { host::RoInitialize(host::RO_INIT_MULTITHREADED).ok()? };
    let owned = HostInitialization(true);
    let dpi = unsafe { host::GetThreadDpiAwarenessContext() };
    let mut dispatcher = HostDispatcher::new()?;
    let adapter = Adapter::load()?;
    let baseline = adapter.image_owners()?;
    let drop_errors = DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire);
    let mut retained = Vec::new();
    for cycle in 0..20_u64 {
        let mut fixture = adapter.render_fixture(96, 64)?;
        let mut capture: FramePoolCapture<Image> = FramePoolCapture::new(fixture.hwnd());
        let mut converter = adapter.converter(0);
        let attempt = capture.capture(&mut converter);
        println!("FramePool lifecycle={cycle} {:?}", attempt.evidence);
        assert!(attempt.evidence.cleanup.iter().all(Option::is_none));
        assert!(!attempt.evidence.replayed);
        assert_eq!(attempt.evidence.target_copy_bytes, 96 * 64 * 4);
        let image = attempt
            .image
            .map_err(|error| format!("cycle {cycle}: {error:?}"))?;
        assert_eq!(image.dimensions(), (96, 64));
        assert_eq!(image.bytes(), expected);
        assert_eq!((converter.conversions, converter.clones), (1, 1));
        assert_ne!(image.address(), capture.cached.as_ref().unwrap().address());
        assert_eq!(adapter.image_owners()?, baseline + cycle + 2);
        assert!(capture.uninit().cleanup.iter().all(Option::is_none));
        assert!(capture.uninit().cleanup.iter().all(Option::is_none));
        assert!(capture.pool.is_none() && capture.session.is_none() && capture.item.is_none());
        assert!(capture.cached.is_some() && capture.device.is_some());
        drop(capture);
        fixture.close()?;
        assert_eq!(adapter.image_owners()?, baseline + cycle + 1);
        assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
        assert_eq!(wgc::EVENT_CLOSE_ERROR.load(Ordering::Acquire), 0);
        assert_eq!(DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire), drop_errors);
        retained.push(image);
        for image in &retained {
            assert_eq!(image.bytes(), expected);
        }
    }
    drop(adapter);
    dispatcher.close()?;
    assert!(
        unsafe { host::AreDpiAwarenessContextsEqual(dpi, host::GetThreadDpiAwarenessContext()) }
            .as_bool()
    );
    drop(owned);
    for image in &retained {
        assert_eq!(image.bytes(), expected);
    }
    let last = retained.pop().unwrap();
    drop(retained);
    assert_eq!(last.native_owners(), baseline + 1);
    assert_eq!(last.bytes(), expected);
    println!(
        "twenty FramePool capture/session/device/window lifecycles complete; all CPU images retained through shutdown"
    );
    Ok(())
}

#[cfg(feature = "prototype-windows-com")]
#[test]
#[ignore = "explicit real FramePool/Map and native clone exception ownership"]
fn releases_mapping_and_result_owner_when_cache_clone_throws()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::parity_adapter::{Adapter, Image};
    let adapter = Adapter::load()?;
    let baseline = adapter.image_owners()?;
    let mut fixture = adapter.render_fixture(96, 64)?;
    let mut capture: FramePoolCapture<Image> = FramePoolCapture::new(fixture.hwnd());
    let mut converter = adapter.converter(0);
    converter.clone_fault_at = Some(1);
    let attempt = capture.capture(&mut converter);
    assert!(matches!(attempt.image, Err(Error::Conversion(2))));
    assert!(attempt.evidence.cleanup.iter().all(Option::is_none));
    assert!(attempt.evidence.target_copy_bytes > 0);
    assert_eq!((converter.conversions, converter.clones), (1, 1));
    assert!(capture.cached.is_none());
    assert_eq!(
        adapter.image_owners()?,
        baseline,
        "temporary result must be released on clone exception"
    );
    {
        let context = capture.context.as_ref().unwrap();
        let texture = capture.readable.as_ref().unwrap();
        let mut mapping = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: Real owned CPU-readable staging texture after the error scope
        // has ended. Observe another native read mapping; exact Unmap call-count
        // interception remains a separate, still-pending boundary check.
        unsafe {
            context
                .Map(texture, 0, D3D11_MAP_READ, 0, Some(&mut mapping))
                .ok()?;
        }
        let _mapping = Mapping { context, texture };
        assert!(!mapping.pData.is_null());
    }
    assert!(capture.uninit().cleanup.iter().all(Option::is_none));
    drop(capture);
    fixture.close()?;
    assert_eq!(adapter.image_owners()?, baseline);
    assert_eq!(
        wgc::LIVE_COMPLETIONS.load(std::sync::atomic::Ordering::Acquire),
        0
    );
    println!(
        "native cache-clone exception: result owner released, staging Map succeeds again, session/pool close succeeds"
    );
    Ok(())
}
