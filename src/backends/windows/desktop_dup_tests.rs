// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;

#[cfg(feature = "prototype-windows")]
#[derive(Debug)]
enum ProbeError {
    Native(&'static str, HRESULT),
    Conversion(i32),
}

#[cfg(feature = "prototype-windows")]
#[test]
#[ignore = "explicit native frame-release and staging-read probes inside conversion, including a contained C++ exception"]
fn releases_acquired_frame_before_external_conversion_and_conversion_exception()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::parity_adapter::{Adapter, Converter};
    struct Probe<'a> {
        inner: Converter<'a>,
        duplication: IDXGIOutputDuplication,
        context: ID3D11DeviceContext,
        staging: ID3D11Texture2D,
        calls: usize,
    }
    impl BgraConverter for Probe<'_> {
        type Image = <Converter<'static> as BgraConverter>::Image;
        type Error = ProbeError;
        fn bgra_to_bgr(&mut self, input: BgraView<'_>) -> Result<Self::Image, Self::Error> {
            self.calls += 1;
            // SAFETY: Real owned duplication, on the same caller. The documented
            // INVALID_CALL result distinguishes an already released native frame.
            let released =
                unsafe { (self.duplication.vtable().ReleaseFrame)(self.duplication.as_raw()) };
            println!("conversion native ReleaseFrame probe={released:?}");
            if released != DXGI_ERROR_INVALID_CALL {
                return Err(ProbeError::Native(
                    "frame still acquired at conversion",
                    released,
                ));
            }
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            // SAFETY: Owned CPU-readable staging and context. This is a separate
            // diagnostic Map at the conversion boundary, not a capture retry.
            let code = unsafe {
                self.context
                    .Map(&self.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            };
            println!("conversion staging Map probe={code:?}");
            if code.is_err() {
                return Err(ProbeError::Native("conversion staging Map", code));
            }
            {
                let _mapping = Mapping {
                    context: &self.context,
                    texture: &self.staging,
                };
                assert!(!mapped.pData.is_null());
                assert_ne!(
                    mapped.pData.cast_const().cast::<u8>(),
                    input.pixels.as_ptr()
                );
                let row = usize::try_from(input.width)
                    .unwrap()
                    .checked_mul(4)
                    .unwrap();
                assert!(usize::try_from(mapped.RowPitch).unwrap() >= row);
                // SAFETY: This successful read map supplies the full first row.
                let native_row =
                    unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>(), row) };
                assert_eq!(&input.pixels[..row], native_row);
            } // diagnostic mapping also ends before invoking C++ conversion
            // A new Map succeeding is not exact Unmap-call-count interception.
            self.inner
                .bgra_to_bgr(input)
                .map_err(ProbeError::Conversion)
        }
    }
    let adapter = Adapter::load()?;
    let baseline = adapter.image_owners()?;
    let mut fixture = adapter.render_fixture(96, 64)?;
    fixture.start_desktop_updates()?;
    let mut capture = DesktopDupCapture::new(fixture.hwnd());
    capture
        .init()
        .map_err(|error| format!("diagnostic init {error:?}"))?;
    capture
        .ensure_output(&mut Evidence::default())
        .map_err(|error| format!("diagnostic output {error:?}"))?;
    // Diagnostic prelude only: obtain a real staging texture for independent
    // interface owners. The subsequent capture keeps its original warmup flag.
    let mut prelude = Evidence::default();
    drop(
        capture
            .sample(&mut prelude)
            .map_err(|error| format!("diagnostic prelude {error:?}"))?,
    );
    assert_eq!(prelude.samples[0].release, Some(HRESULT(0)));
    let mut probe = Probe {
        inner: adapter.converter(0),
        duplication: capture.duplication.as_ref().unwrap().clone(),
        context: capture.context.as_ref().unwrap().clone(),
        staging: capture.readable.as_ref().unwrap().clone(),
        calls: 0,
    };
    let first = capture.capture(&mut probe);
    println!("conversion successful capture {:?}", first.evidence);
    for sample in &first.evidence.samples[..first.evidence.sample_count] {
        assert_eq!(sample.acquire, Some(HRESULT(0)));
        assert_eq!(sample.release, Some(HRESULT(0)));
    }
    let first = first
        .image
        .map_err(|error| format!("conversion probe {error:?}"))?;
    assert_eq!(probe.calls, 1);
    let saved = first.bytes().to_vec();
    assert_eq!(adapter.image_owners()?, baseline + 1);
    probe.inner = adapter.converter(1); // real contained cv::Exception
    let thrown = capture.capture(&mut probe);
    println!("conversion exceptional capture {:?}", thrown.evidence);
    assert!(matches!(
        thrown.image,
        Err(Error::Conversion(ProbeError::Conversion(2)))
    ));
    assert_eq!(probe.calls, 2);
    for sample in &thrown.evidence.samples[..thrown.evidence.sample_count] {
        assert_eq!(sample.acquire, Some(HRESULT(0)));
        assert_eq!(sample.release, Some(HRESULT(0)));
    }
    assert_eq!(adapter.image_owners()?, baseline + 1);
    drop(probe);
    drop(capture);
    fixture.close()?;
    assert_eq!(first.bytes(), saved);
    Ok(())
}

#[cfg(feature = "prototype-windows")]
#[test]
#[ignore = "explicit null-HWND adapter-zero/output-zero source pixels and native desktop"]
fn selects_adapter_zero_output_zero_for_null_hwnd_and_retains_cpu_pixels()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::parity_adapter::Adapter;
    let adapter = Adapter::load()?;
    let baseline = adapter.image_owners()?;
    let mut fixture = adapter.render_fixture(96, 64)?;
    fixture.start_desktop_updates()?;
    let mut origin = POINT::default();
    // SAFETY: Live harness-owned window, exact writable point on this caller.
    assert!(unsafe { ClientToScreen(HWND(fixture.hwnd()), &mut origin) }.as_bool());
    let bounds = {
        // Independent preflight owns and drops its own native references. It
        // does not initialize or change the capture owner's selection state.
        // SAFETY: Exact generated IID returns one owned factory reference.
        let factory: IDXGIFactory = unsafe { CreateDXGIFactory() }?;
        // SAFETY: Live factory, owned returned reference for native index zero.
        let selected_adapter = unsafe { factory.EnumAdapters(0) }?;
        let mut output = None;
        // SAFETY: Live adapter and initialized owned output slot.
        unsafe { selected_adapter.EnumOutputs(0, &mut output) }.ok()?;
        let output = output.ok_or("default output is null")?;
        let mut desc = DXGI_OUTPUT_DESC::default();
        // SAFETY: Live output and exact writable descriptor on this caller.
        unsafe { output.GetDesc(&mut desc) }.ok()?;
        let r = desc.DesktopCoordinates;
        println!(
            "default output preflight bounds={r:?} rotation={:?} attached={:?} fixture={origin:?}",
            desc.Rotation, desc.AttachedToDesktop
        );
        if !desc.AttachedToDesktop.as_bool()
            || origin.x < r.left
            || origin.y < r.top
            || i64::from(origin.x) + 96 > i64::from(r.right)
            || i64::from(origin.y) + 64 > i64::from(r.bottom)
        {
            return Err("MFS_MISSING_ENVIRONMENT:default_output_fixture".into());
        }
        [r.left, r.top, r.right, r.bottom]
    };
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_DESKTOP_DUP_REFERENCE_ROOT")
            .ok_or("original desktop records required")?,
    );
    if !root.is_absolute() {
        return Err("original desktop record root must be absolute".into());
    }
    let source = std::fs::read(root.join("default-output/target.bgr"))?;
    assert_eq!(source.len(), 96 * 64 * 3);
    let mut converter = adapter.converter(0);
    let mut capture = DesktopDupCapture::new(std::ptr::null_mut());
    let result = capture.capture(&mut converter);
    println!("null HWND capture {:?}", result.evidence);
    let image = result
        .image
        .map_err(|error| format!("null HWND capture {error:?}"))?;
    assert!(result.evidence.primary_fallback);
    assert!(!result.evidence.monitor_query_failed);
    assert_eq!(result.evidence.adapters_seen, 0);
    assert_eq!(result.evidence.outputs_seen, 0);
    assert_eq!(capture.output_coordinates().unwrap(), bounds);
    assert_eq!(converter.conversions, 1);
    assert!((1..=3).contains(&result.evidence.warmups));
    assert_eq!(result.evidence.sample_count, result.evidence.warmups + 1);
    for sample in &result.evidence.samples[..result.evidence.sample_count] {
        assert_eq!(sample.acquire, Some(HRESULT(0)));
        assert_eq!(sample.release, Some(HRESULT(0)));
        assert!(sample.copied_bytes > 0);
    }
    let x = usize::try_from(i64::from(origin.x) - i64::from(bounds[0]))?;
    let y = usize::try_from(i64::from(origin.y) - i64::from(bounds[1]))?;
    for row in 0..64_usize {
        let offset = (y + row) * image.stride() + x * 3;
        let actual = &image.bytes()[offset..offset + 96 * 3];
        assert_eq!(actual, &source[row * 96 * 3..(row + 1) * 96 * 3]);
        for column in 0..96_usize {
            let expected = if column < 32 {
                [0x1f, 0x53, 0xa7]
            } else if row < 32 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            assert_eq!(&actual[column * 3..column * 3 + 3], expected);
        }
    }
    assert_eq!(adapter.image_owners()?, baseline + 1);
    let saved = image.bytes().to_vec();
    drop(capture);
    fixture.close()?;
    assert_eq!(image.bytes(), saved);
    drop(image);
    assert_eq!(adapter.image_owners()?, baseline);
    Ok(())
}

#[cfg(all(feature = "prototype-windows", feature = "windows-desktop-dup-window"))]
#[test]
#[ignore = "explicit original DesktopDup/Window images and real native update fixture"]
fn matches_original_target_pixels_across_twenty_desktop_and_window_lifecycles()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::{
        desktop_dup_window::DesktopDupWindowCapture, parity_adapter::Adapter,
    };
    let reference = std::path::PathBuf::from(
        std::env::var_os("MFS_DESKTOP_DUP_REFERENCE_ROOT")
            .ok_or("original desktop records required")?,
    );
    if !reference.is_absolute() {
        return Err("original desktop record root must be absolute".into());
    }
    let desktop = std::fs::read(reference.join("desktop/target.bgr"))?;
    let window = std::fs::read(reference.join("window/target.bgr"))?;
    assert_eq!(desktop.len(), 96 * 64 * 3);
    assert_eq!(window, desktop);
    let adapter = Adapter::load()?;
    let baseline = adapter.image_owners()?;
    let mut retained = Vec::new();
    for cycle in 0..20 {
        let mut fixture = adapter.render_fixture(96, 64)?;
        fixture.start_desktop_updates()?;
        let mut converter = adapter.converter(0);
        let (image, evidence) = if cycle % 2 == 0 {
            let mut capture = DesktopDupCapture::new(fixture.hwnd());
            let attempt = capture.capture(&mut converter);
            println!("DesktopDup full cycle={cycle} {:?}", attempt.evidence);
            let image = attempt
                .image
                .map_err(|error| format!("desktop cycle={cycle}: {error:?}"))?;
            let bounds = capture
                .output_coordinates()
                .map_err(|error| format!("output {error:?}"))?;
            let mut origin = POINT::default();
            // SAFETY: Live independently owned fixture HWND and exact output.
            assert!(unsafe { ClientToScreen(HWND(fixture.hwnd()), &mut origin) }.as_bool());
            let x = usize::try_from(origin.x.checked_sub(bounds[0]).ok_or("x overflow")?)?;
            let y = usize::try_from(origin.y.checked_sub(bounds[1]).ok_or("y overflow")?)?;
            for row in 0..64_usize {
                let offset = (y + row) * image.stride() + x * 3;
                assert_eq!(
                    image.bytes()[offset..offset + 96 * 3],
                    desktop[row * 96 * 3..(row + 1) * 96 * 3]
                );
            }
            drop(capture);
            (image, attempt.evidence)
        } else {
            let mut capture = DesktopDupWindowCapture::new(fixture.hwnd());
            let attempt = capture.capture(&mut converter);
            println!("DesktopDup window cycle={cycle} {:?}", attempt.evidence);
            assert!(attempt.coordination.as_ref().unwrap().geometry.is_ok());
            assert_eq!(attempt.target_copy_bytes, 96 * 64 * 3);
            let image = attempt
                .image
                .map_err(|error| format!("window cycle={cycle}: {error:?}"))?;
            assert_eq!(image.dimensions(), (96, 64));
            assert_eq!(image.stride(), 96 * 3);
            assert_eq!(image.bytes(), window);
            drop(capture);
            (image, attempt.evidence)
        };
        assert_eq!(converter.conversions, 1);
        assert!((1..=3).contains(&evidence.warmups));
        assert_eq!(evidence.sample_count, evidence.warmups + 1);
        for sample in &evidence.samples[..evidence.sample_count] {
            assert_eq!(sample.acquire, Some(HRESULT(0)));
            assert_eq!(sample.release, Some(HRESULT(0)));
            assert!(sample.copied_bytes > 0);
        }
        let marker = fixture.desktop_update_info()?;
        assert!(marker[0] != 0 && marker[1] > 1);
        assert_eq!(marker[2], 0);
        fixture.close()?;
        assert_eq!(
            adapter.image_owners()?,
            baseline + u64::try_from(cycle)? + 1
        );
        let saved = image.bytes().to_vec();
        retained.push((image, saved));
    }
    drop(adapter);
    for (image, saved) in retained {
        assert_eq!(image.bytes(), saved);
    }
    Ok(())
}
#[test]
fn copies_defined_desktop_rows_without_mapped_padding_or_native_borrows() {
    let mut bytes = [0xd7_u8; 48];
    bytes[0..8].copy_from_slice(&[1, 2, 3, 255, 4, 5, 6, 255]);
    bytes[24..32].copy_from_slice(&[7, 8, 9, 255, 10, 11, 12, 255]);
    let desc = D3D11_TEXTURE2D_DESC {
        Width: 2,
        Height: 2,
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
        RowPitch: 24,
        DepthPitch: 48,
    };
    // SAFETY: Owned initialized storage provides each declared native row.
    let raw = unsafe { copy_desktop(&mapped, desc) }.unwrap();
    assert_eq!(
        raw.bytes,
        [1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255]
    );
    bytes.fill(0);
    assert_eq!(raw.bytes[0..4], [1, 2, 3, 255]);
    let invalid = D3D11_MAPPED_SUBRESOURCE {
        RowPitch: 7,
        ..mapped
    };
    assert!(matches!(
        unsafe { copy_desktop(&invalid, desc) },
        Err(CaptureError::InvalidLayout)
    ));
}
