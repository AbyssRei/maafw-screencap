// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::windows::parity_adapter::{Adapter, Image};
use std::error::Error;

fn assert_fixture(image: &Image, phase: usize, width: i32, height: i32) {
    assert_eq!(image.dimensions(), (width, height));
    let width = usize::try_from(width).unwrap();
    let height = usize::try_from(height).unwrap();
    let palette = [
        [[0x1f, 0x53, 0xa7], [0xb9, 0x4d, 0x27], [0x3b, 0xe2, 0x81]],
        [[0xe3, 0x42, 0x19], [0x57, 0x8b, 0xc1], [0x9d, 0x23, 0xf5]],
    ];
    for y in 0..height {
        for x in 0..width {
            let color = if x < width / 3 { 0 } else if y < height / 2 { 1 } else { 2 };
            let offset = y.checked_mul(image.stride()).unwrap().checked_add(x.checked_mul(3).unwrap()).unwrap();
            assert_eq!(&image.bytes()[offset..offset + 3], &palette[phase][color], "pixel ({x},{y})");
        }
    }
}

#[test]
#[ignore = "explicit native desktop/reference parity; run cargo xtask upstream-parity gdi"]
fn matches_unpatched_gdi_for_odd_width_asymmetric_pixels_and_retained_outputs() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    for (width, height) in [(37, 23), (64, 31)] {
        let mut window = adapter.window(width, height).map_err(|status| format!("window fixture: {status}"))?;
        let (width, height) = window.dimensions();
        let baseline = adapter.objects();
        let mut capture = GdiCapture::new(window.hwnd());
        let mut converter = adapter.converter(0);
        let first = capture.capture(&mut converter);
        assert!(first.cleanup.0.iter().all(Option::is_none), "{:?}", first.cleanup);
        let first = first.image.map_err(|error| format!("Rust GDI: {error:?}"))?;
        assert_fixture(&first, 0, width, height);
        assert!(converter.observed_objects.unwrap() >= baseline.checked_add(2).unwrap(), "conversion must precede DC/bitmap cleanup");
        assert_eq!(adapter.objects(), baseline, "Rust GDI cleanup");
        for cycle in 0..20_u32 {
            let phase = cycle % 2;
            window.paint(phase).map_err(|status| format!("paint fixture: {status}"))?;
            let expected = adapter.reference(window.hwnd()).map_err(|status| format!("original GDI: {status}"))?;
            let actual = capture.capture(&mut converter);
            assert!(actual.cleanup.0.iter().all(Option::is_none), "{:?}", actual.cleanup);
            let actual = actual.image.map_err(|error| format!("Rust GDI: {error:?}"))?;
            assert_fixture(&expected, usize::try_from(phase).unwrap(), width, height);
            assert_fixture(&actual, usize::try_from(phase).unwrap(), width, height);
            assert_eq!(actual.bytes(), expected.bytes(), "original/Rust GDI cycle {cycle}");
            assert_fixture(&first, 0, width, height);
            assert_eq!(adapter.objects(), baseline, "original/Rust GDI resource balance");
        }
        window.close().map_err(|status| format!("destroy fixture: {status}"))?;
        assert_fixture(&first, 0, width, height);
        println!("GDI parity: {width}x{height}, 20 cycles, all BGR pixels and retained output verified");
    }
    Ok(())
}

#[test]
#[ignore = "explicit native reference/failure parity; run cargo xtask upstream-parity gdi"]
fn rejects_null_destroyed_and_empty_windows_and_cleans_up_conversion_failure() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let mut converter = adapter.converter(0);
    let baseline = adapter.objects();
    let null = GdiCapture::new(null_mut()).capture(&mut converter);
    assert!(matches!(null.image, Err(CaptureError::NullWindow)));
    assert!(null.cleanup.0.iter().all(Option::is_none));
    assert!(converter.observed_objects.is_none());
    assert!(matches!(adapter.reference(null_mut()), Err(1)));
    assert_eq!(adapter.objects(), baseline);

    let mut empty = adapter.window(0, 0).map_err(|status| format!("empty fixture: {status}"))?;
    let empty_baseline = adapter.objects();
    let result = GdiCapture::new(empty.hwnd()).capture(&mut converter);
    assert!(matches!(result.image, Err(CaptureError::InvalidDimensions)));
    assert!(result.cleanup.0.iter().all(Option::is_none));
    assert!(matches!(adapter.reference(empty.hwnd()), Err(1)));
    assert_eq!(adapter.objects(), empty_baseline);
    empty.close().map_err(|status| format!("close empty fixture: {status}"))?;

    let mut window = adapter.window(37, 23).map_err(|status| format!("window fixture: {status}"))?;
    let baseline = adapter.objects();
    let mut capture = GdiCapture::new(window.hwnd());
    let failure = capture.capture(&mut adapter.converter(1));
    assert!(matches!(failure.image, Err(CaptureError::Conversion(2))));
    assert!(failure.cleanup.0.iter().all(Option::is_none));
    assert_eq!(adapter.objects(), baseline, "cleanup after contained OpenCV exception");
    window.close().map_err(|status| format!("close fixture: {status}"))?;
    let dead = capture.capture(&mut converter);
    assert!(dead.image.is_err());
    assert!(dead.cleanup.0.iter().all(Option::is_none));
    // The original fixture HWND has been destroyed, with no intervening window
    // creation in this process. Both callers must reject the stale handle.
    assert!(matches!(adapter.reference(capture.hwnd), Err(1)));
    Ok(())
}
