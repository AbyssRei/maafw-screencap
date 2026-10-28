// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::windows::parity_adapter::probe as p;
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
            let color = if x < width / 3 {
                0
            } else if y < height / 2 {
                1
            } else {
                2
            };
            let offset = y
                .checked_mul(image.stride())
                .unwrap()
                .checked_add(x.checked_mul(3).unwrap())
                .unwrap();
            assert_eq!(
                &image.bytes()[offset..offset + 3],
                &palette[phase][color],
                "pixel ({x},{y})"
            );
        }
    }
}

#[test]
#[ignore = "explicit native desktop/reference parity; run cargo xtask upstream-parity gdi"]
fn matches_unpatched_gdi_for_odd_width_asymmetric_pixels_and_retained_outputs()
-> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    for (width, height) in [(37, 23), (64, 31)] {
        let mut window = adapter
            .window(width, height)
            .map_err(|status| format!("window fixture: {status}"))?;
        let (width, height) = window.dimensions();
        let baseline = adapter.objects();
        let mut capture = GdiCapture::new(window.hwnd());
        let mut converter = adapter.converter(0);
        let first = capture.capture(&mut converter);
        assert!(
            first.cleanup.0.iter().all(Option::is_none),
            "{:?}",
            first.cleanup
        );
        let first = first
            .image
            .map_err(|error| format!("Rust GDI: {error:?}"))?;
        assert_fixture(&first, 0, width, height);
        assert!(
            converter.observed_objects.unwrap() >= baseline.checked_add(2).unwrap(),
            "conversion must precede DC/bitmap cleanup"
        );
        assert_eq!(adapter.objects(), baseline, "Rust GDI cleanup");
        for cycle in 0..20_u32 {
            let phase = cycle % 2;
            window
                .paint(phase)
                .map_err(|status| format!("paint fixture: {status}"))?;
            let expected = adapter
                .reference(window.hwnd())
                .map_err(|status| format!("original GDI: {status}"))?;
            let actual = capture.capture(&mut converter);
            assert!(
                actual.cleanup.0.iter().all(Option::is_none),
                "{:?}",
                actual.cleanup
            );
            let actual = actual
                .image
                .map_err(|error| format!("Rust GDI: {error:?}"))?;
            assert_fixture(&expected, usize::try_from(phase).unwrap(), width, height);
            assert_fixture(&actual, usize::try_from(phase).unwrap(), width, height);
            assert_eq!(
                actual.bytes(),
                expected.bytes(),
                "original/Rust GDI cycle {cycle}"
            );
            assert_fixture(&first, 0, width, height);
            assert_eq!(
                adapter.objects(),
                baseline,
                "original/Rust GDI resource balance"
            );
        }
        window
            .close()
            .map_err(|status| format!("destroy fixture: {status}"))?;
        assert_fixture(&first, 0, width, height);
        println!(
            "GDI parity: {width}x{height}, 20 cycles, all BGR pixels and retained output verified"
        );
    }
    Ok(())
}

#[test]
#[ignore = "explicit native reference/failure parity; run cargo xtask upstream-parity gdi"]
fn rejects_null_destroyed_and_empty_windows_and_cleans_up_conversion_failure()
-> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let mut converter = adapter.converter(0);
    let baseline = adapter.objects();
    let null = GdiCapture::new(null_mut()).capture(&mut converter);
    assert!(matches!(null.image, Err(CaptureError::NullWindow)));
    assert!(null.cleanup.0.iter().all(Option::is_none));
    assert!(converter.observed_objects.is_none());
    assert!(matches!(adapter.reference(null_mut()), Err(1)));
    assert_eq!(adapter.objects(), baseline);

    let mut empty = adapter
        .window(0, 0)
        .map_err(|status| format!("empty fixture: {status}"))?;
    let empty_baseline = adapter.objects();
    let result = GdiCapture::new(empty.hwnd()).capture(&mut converter);
    assert!(matches!(result.image, Err(CaptureError::InvalidDimensions)));
    assert!(result.cleanup.0.iter().all(Option::is_none));
    assert!(matches!(adapter.reference(empty.hwnd()), Err(1)));
    assert_eq!(adapter.objects(), empty_baseline);
    empty
        .close()
        .map_err(|status| format!("close empty fixture: {status}"))?;

    let mut window = adapter
        .window(37, 23)
        .map_err(|status| format!("window fixture: {status}"))?;
    let baseline = adapter.objects();
    let mut capture = GdiCapture::new(window.hwnd());
    let failure = capture.capture(&mut adapter.converter(1));
    assert!(matches!(failure.image, Err(CaptureError::Conversion(2))));
    assert!(failure.cleanup.0.iter().all(Option::is_none));
    assert_eq!(
        adapter.objects(),
        baseline,
        "cleanup after contained OpenCV exception"
    );
    window
        .close()
        .map_err(|status| format!("close fixture: {status}"))?;
    let dead = capture.capture(&mut converter);
    assert!(dead.image.is_err());
    assert!(dead.cleanup.0.iter().all(Option::is_none));
    // The original fixture HWND has been destroyed, with no intervening window
    // creation in this process. Both callers must reject the stale handle.
    assert!(matches!(adapter.reference(capture.hwnd), Err(1)));
    Ok(())
}

#[test]
#[ignore = "explicit native fault/reference parity; run cargo xtask upstream-parity gdi"]
fn matches_source_native_failures_and_preserves_primary_plus_cleanup_evidence()
-> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let mut window = adapter
        .window(37, 23)
        .map_err(|status| format!("fixture: {status}"))?;
    let mut capture = GdiCapture::new(window.hwnd());
    let mut masks = vec![0];
    masks.extend(
        [
            p::GET_DC,
            p::CREATE_DC,
            p::CLIENT_RECT,
            p::CREATE_BITMAP,
            p::SELECT,
            p::COPY,
            p::READ,
            p::CONVERT,
            p::RESTORE,
            p::DELETE_BITMAP,
            p::DELETE_DC,
            p::RELEASE_DC,
        ]
        .map(p::fault),
    );
    masks.push(
        p::fault(p::COPY)
            | p::fault(p::RESTORE)
            | p::fault(p::DELETE_BITMAP)
            | p::fault(p::DELETE_DC)
            | p::fault(p::RELEASE_DC),
    );
    for mask in masks {
        let owners_before = adapter.image_owners()?;
        let (expected, original_report) = adapter.reference_probe(window.hwnd(), mask)?;
        original_report.check_fixture();
        let probe = adapter.probe(mask)?;
        let actual = capture.capture_with_api(&probe, &mut adapter.converter(0));
        let report = probe.finish()?;
        report.check_fixture();
        assert_eq!(actual.image.is_ok(), expected.is_ok(), "mask={mask:#x}");
        assert_eq!(
            report.comparable(),
            original_report.comparable(),
            "native order/results mask={mask:#x}"
        );
        assert_eq!(
            i64::from(report.after_capture) - i64::from(report.before),
            i64::from(original_report.after_capture) - i64::from(original_report.before),
            "residual native resources mask={mask:#x}"
        );
        for (slot, operation) in [p::RESTORE, p::DELETE_BITMAP, p::DELETE_DC, p::RELEASE_DC]
            .into_iter()
            .enumerate()
        {
            let failure = report
                .events()
                .iter()
                .find(|event| event.operation == operation && event.result == 0);
            assert_eq!(
                actual.cleanup.0[slot].is_some(),
                failure.is_some(),
                "cleanup slot {slot}, mask={mask:#x}"
            );
            if let Some(event) = failure {
                assert_eq!(actual.cleanup.0[slot].unwrap().last_error, event.last_error);
            }
        }
        if mask & p::fault(p::COPY) != 0 {
            assert!(
                matches!(actual.image, Err(CaptureError::Native(NativeFailure { operation: "BitBlt", last_error })) if last_error == 0xe0f00000 | p::COPY)
            );
        }
        if let (Ok(actual), Ok(expected)) = (&actual.image, &expected) {
            assert_fixture(actual, 0, 37, 23);
            assert_eq!(actual.bytes(), expected.bytes());
        }
        println!(
            "GDI failure parity: mask={mask:#010x}, {} events, native residual delta={} before separate fixture cleanup",
            report.count,
            i64::from(report.after_capture) - i64::from(report.before)
        );
        drop(actual);
        drop(expected);
        assert_eq!(
            adapter.image_owners()?,
            owners_before,
            "opaque CPU image owners must release once"
        );
    }
    window
        .close()
        .map_err(|status| format!("close fixture: {status}"))?;
    Ok(())
}

#[test]
#[ignore = "explicit safe defect observation; run cargo xtask upstream-parity gdi"]
fn classifies_original_short_read_and_24bit_assumptions_without_undefined_input()
-> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let mut window = adapter
        .window(37, 23)
        .map_err(|status| format!("fixture: {status}"))?;
    let mut capture = GdiCapture::new(window.hwnd());
    for mask in [p::SHORT_READ, p::BITMAP_24, p::fault(p::OBJECT)] {
        let owners_before = adapter.image_owners()?;
        let (expected, original_report) = adapter.reference_probe(window.hwnd(), mask)?;
        original_report.check_fixture();
        let probe = adapter.probe(mask)?;
        let actual = capture.capture_with_api(&probe, &mut adapter.converter(0));
        let report = probe.finish()?;
        report.check_fixture();
        assert!(actual.cleanup.0.iter().all(Option::is_none));
        assert!(
            !report
                .events()
                .iter()
                .any(|event| event.operation == p::CONVERT)
        );
        match mask {
            p::SHORT_READ => {
                assert!(
                    expected.is_ok(),
                    "the initialized reference accepts a positive short return"
                );
                assert!(matches!(
                    actual.image,
                    Err(CaptureError::IncompleteRead {
                        copied: 1702,
                        required: 3404
                    })
                ));
                assert_fixture(expected.as_ref().ok().unwrap(), 0, 37, 23);
            }
            p::BITMAP_24 => {
                assert_eq!(original_report.bitmap_depth, 24);
                assert_eq!(report.bitmap_depth, 24);
                if expected.is_ok() {
                    // Some devices accept a 24-bit bitmap. The instrumented
                    // input tail is defined; Rust rejects its actual metadata.
                    assert!(matches!(actual.image, Err(CaptureError::UnsupportedBitmap)));
                } else {
                    // This 32-bit device rejects the real 24-bit DDB at select.
                    // This is not an actual nondefault-depth display capture.
                    assert!(
                        original_report
                            .events()
                            .iter()
                            .any(|event| event.operation == p::SELECT && event.result == 0)
                    );
                    assert!(matches!(
                        actual.image,
                        Err(CaptureError::Native(NativeFailure {
                            operation: "SelectObject",
                            ..
                        }))
                    ));
                    assert_eq!(report.comparable(), original_report.comparable());
                }
            }
            _ => {
                assert!(
                    expected.is_ok(),
                    "the original does not perform the Rust metadata query"
                );
                assert!(
                    matches!(actual.image, Err(CaptureError::Native(NativeFailure { operation: "GetObjectW", last_error })) if last_error == 0xe0f00000 | p::OBJECT)
                );
            }
        }
        println!(
            "GDI safety classification: mask={mask:#010x}, bitmap_depth={}, original sample_success={}, Rust sample_success={}, reference tail initialized if read is reached",
            report.bitmap_depth,
            expected.is_ok(),
            actual.image.is_ok()
        );
        drop(actual);
        drop(expected);
        assert_eq!(adapter.image_owners()?, owners_before);
    }
    window
        .close()
        .map_err(|status| format!("close fixture: {status}"))?;
    Ok(())
}

#[test]
#[ignore = "explicit owned-window state parity; run cargo xtask upstream-parity gdi"]
fn preserves_original_iconic_window_outcome() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let mut window = adapter
        .window(37, 23)
        .map_err(|status| format!("fixture: {status}"))?;
    window.minimize()?;
    let baseline = adapter.objects();
    let expected = adapter.reference(window.hwnd());
    let actual = GdiCapture::new(window.hwnd()).capture(&mut adapter.converter(0));
    assert!(actual.cleanup.0.iter().all(Option::is_none));
    assert_eq!(actual.image.is_ok(), expected.is_ok());
    if let (Ok(actual), Ok(expected)) = (&actual.image, &expected) {
        assert_eq!(actual.dimensions(), expected.dimensions());
        assert_eq!(actual.bytes(), expected.bytes());
    }
    assert_eq!(adapter.objects(), baseline);
    println!(
        "GDI iconic window: original/Rust sample_success={}",
        expected.is_ok()
    );
    window
        .close()
        .map_err(|status| format!("close fixture: {status}"))?;
    Ok(())
}
