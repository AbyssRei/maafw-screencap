// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;

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
