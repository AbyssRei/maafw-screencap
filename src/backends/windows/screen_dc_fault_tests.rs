// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::windows::parity_adapter::{Adapter, probe as p};

#[test]
#[ignore = "native foreground helper-only cold-process resource control; no capture"]
fn observes_foreground_setup_resource_initialization_without_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let adapter = Adapter::load()?;
    let mut empty = adapter
        .window(0, 23)
        .map_err(|s| format!("empty fixture {s}"))?;
    let before = adapter.objects();
    let foreground = crate::platform::windows::foreground::ensure_foreground(empty.hwnd());
    let after = adapter.objects();
    empty.close().map_err(|s| format!("empty close {s}"))?;
    println!(
        "foreground-helper-only zero-width fixture: before={before}, after={after}, closed={}, foreground={foreground}; no capture/DC/bitmap implementation invoked",
        adapter.objects()
    );
    assert_eq!(adapter.objects(), after);
    Ok(())
}

#[test]
#[ignore = "explicit native null/destroyed/empty target reference; no visible pixel inference"]
fn rejects_native_null_destroyed_and_empty_client_targets_without_acquiring_capture_resources()
-> Result<(), Box<dyn std::error::Error>> {
    let adapter = Adapter::load()?;
    let mut converter = adapter.converter(0);
    let baseline = adapter.objects();
    assert!(adapter.screen_reference(null_mut()).is_err());
    let null = ScreenDcCapture::new(null_mut()).capture(&mut converter);
    assert!(matches!(null.image, Err(CaptureError::NullWindow)));
    assert!(null.coordination.is_none());
    assert!(null.cleanup.0.iter().all(Option::is_none));
    assert_eq!(adapter.objects(), baseline);
    assert!(converter.observed_objects.is_none());

    let mut empty = adapter
        .window(0, 23)
        .map_err(|s| format!("empty fixture {s}"))?;
    // A separate helper-only cold-process control observes 0 -> 7 GDI objects
    // without invoking capture. Initialize that owned-window setup before
    // measuring whether the capture preflight acquires/leaks any resources.
    let _ = crate::platform::windows::foreground::ensure_foreground(empty.hwnd());
    let initialized = adapter.objects();
    assert!(adapter.screen_reference(empty.hwnd()).is_err());
    let result = ScreenDcCapture::new(empty.hwnd()).capture(&mut converter);
    assert!(matches!(result.image, Err(CaptureError::InvalidDimensions)));
    assert!(result.coordination.is_some());
    assert!(result.cleanup.0.iter().all(Option::is_none));
    assert_eq!(adapter.objects(), initialized);
    assert!(converter.observed_objects.is_none());
    empty.close().map_err(|s| format!("empty close {s}"))?;

    let mut window = adapter.screen_fixture()?;
    let stale = window.hwnd();
    window.close()?;
    let closed = adapter.objects();
    // No subsequent native window creation can reuse the fixture's HWND.
    assert!(adapter.screen_reference(stale).is_err());
    let result = ScreenDcCapture::new(stale).capture(&mut converter);
    assert!(matches!(
        result.image,
        Err(CaptureError::Native(NativeFailure {
            operation: "GetWindowRect",
            ..
        }))
    ));
    let warnings = result.coordination.unwrap();
    assert!(!warnings.foreground);
    assert_eq!(warnings.geometry, Err(geometry::Error::InvalidWindow));
    assert!(result.cleanup.0.iter().all(Option::is_none));
    assert_eq!(adapter.objects(), closed);
    assert!(converter.observed_objects.is_none());
    Ok(())
}

#[test]
#[ignore = "explicit real native resource/failure reference; visible pixel acceptance is separate"]
fn matches_original_screen_dc_failures_and_records_residuals_before_fixture_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let adapter = Adapter::load()?;
    let mut window = adapter.screen_fixture()?;
    let mut capture = ScreenDcCapture::new(window.hwnd());
    let mut masks = vec![0];
    masks.extend(
        [
            p::GET_DC,
            p::CREATE_DC,
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
        let owners = adapter.image_owners()?;
        let (expected, source) = adapter.screen_reference_probe(window.hwnd(), mask)?;
        source.check_fixture();
        let probe = adapter.probe(mask)?;
        let actual = capture.capture_with_api(&probe, &SystemWindow, &mut adapter.converter(0));
        let report = probe.finish()?;
        report.check_fixture();
        assert_eq!(actual.image.is_ok(), expected.is_ok(), "mask={mask:#x}");
        if mask == 0 {
            assert!(
                expected.is_ok(),
                "native acquisition/read/conversion positive control"
            );
        }
        for operation in [
            p::GET_DC,
            p::CREATE_DC,
            p::CREATE_BITMAP,
            p::SELECT,
            p::COPY,
            p::READ,
            p::CONVERT,
            p::RESTORE,
            p::DELETE_BITMAP,
            p::DELETE_DC,
            p::RELEASE_DC,
        ] {
            if mask & p::fault(operation) != 0 {
                assert!(
                    report.events().iter().any(|e| e.operation == operation
                        && e.result == 0
                        && e.last_error == (0xe0f00000 | operation)),
                    "injection must be reached: op={operation}, mask={mask:#x}"
                );
            }
        }
        assert_eq!(
            report.comparable(),
            source.comparable(),
            "native DC/bitmap call/results/error order mask={mask:#x}"
        );
        assert_eq!(
            i64::from(report.after_capture) - i64::from(report.before),
            i64::from(source.after_capture) - i64::from(source.before),
            "residuals before separate fixture recovery mask={mask:#x}"
        );
        for (slot, op) in [p::RESTORE, p::DELETE_BITMAP, p::DELETE_DC, p::RELEASE_DC]
            .into_iter()
            .enumerate()
        {
            let error = report
                .events()
                .iter()
                .find(|e| e.operation == op && e.result == 0);
            assert_eq!(
                actual.cleanup.0[slot].is_some(),
                error.is_some(),
                "slot={slot}, mask={mask:#x}"
            );
            if let Some(error) = error {
                assert_eq!(actual.cleanup.0[slot].unwrap().last_error, error.last_error);
            }
        }
        if mask & p::fault(p::COPY) != 0 {
            assert!(
                matches!(actual.image,Err(CaptureError::Native(NativeFailure{operation:"BitBlt",last_error})) if last_error==0xe0f00000|p::COPY)
            );
        }
        if mask & p::fault(p::CONVERT) != 0 {
            assert!(matches!(actual.image, Err(CaptureError::Conversion(2))));
        }
        if let (Ok(actual), Ok(expected)) = (&actual.image, &expected) {
            assert_eq!(actual.dimensions(), (37, 23));
            assert_eq!(actual.dimensions(), expected.dimensions());
        }
        // No lit-display/known-pixel assertion is inferred from this resource
        // fault ledger. The independent strict visible-pixel case stays required.
        println!(
            "ScreenDC native fault parity: mask={mask:#010x}, events={}, residual_delta={} before fixture recovery",
            report.count,
            i64::from(report.after_capture) - i64::from(report.before)
        );
        drop(actual);
        drop(expected);
        assert_eq!(
            adapter.image_owners()?,
            owners,
            "CPU owner release mask={mask:#x}"
        );
    }
    window.close()?;
    Ok(())
}

#[test]
#[ignore = "explicit native safety classification with defined instrumented reference input"]
fn classifies_short_reads_metadata_failures_and_24bit_device_bitmap_assumptions()
-> Result<(), Box<dyn std::error::Error>> {
    let adapter = Adapter::load()?;
    let mut window = adapter.screen_fixture()?;
    let mut capture = ScreenDcCapture::new(window.hwnd());
    for mask in [p::SHORT_READ, p::BITMAP_24, p::fault(p::OBJECT)] {
        let owners = adapter.image_owners()?;
        let (source, source_report) = adapter.screen_reference_probe(window.hwnd(), mask)?;
        source_report.check_fixture();
        let probe = adapter.probe(mask)?;
        let actual = capture.capture_with_api(&probe, &SystemWindow, &mut adapter.converter(0));
        let report = probe.finish()?;
        report.check_fixture();
        assert_eq!(report.bitmap_depth, source_report.bitmap_depth);
        if mask == p::SHORT_READ {
            assert!(
                source.is_ok(),
                "original checks nonzero, with fully defined diagnostic input"
            );
            assert!(
                matches!(actual.image,Err(CaptureError::IncompleteRead{copied,required}) if copied>0 && copied<required)
            );
            let read = report
                .events()
                .iter()
                .find(|e| e.operation == p::READ)
                .unwrap();
            assert!(read.result > 0);
            assert!(!report.events().iter().any(|e| e.operation == p::CONVERT));
        } else if mask == p::BITMAP_24 {
            assert_eq!(report.bitmap_depth, 24);
            if source.is_ok() {
                assert!(matches!(actual.image, Err(CaptureError::UnsupportedBitmap)));
                assert!(
                    !report
                        .events()
                        .iter()
                        .any(|e| e.operation == p::READ || e.operation == p::CONVERT)
                );
            } else {
                // A 24-bit DDB can be rejected at SelectObject on a 32-bit
                // device. That is not actual nondefault display-depth capture.
                assert!(
                    source_report
                        .events()
                        .iter()
                        .any(|e| e.operation == p::SELECT && e.result == 0)
                );
                assert!(matches!(
                    actual.image,
                    Err(CaptureError::Native(NativeFailure {
                        operation: "SelectObject",
                        ..
                    }))
                ));
                assert_eq!(report.comparable(), source_report.comparable());
            }
        } else {
            assert!(source.is_ok(), "original has no bitmap metadata query");
            assert!(
                matches!(actual.image,Err(CaptureError::Native(NativeFailure{operation:"GetObjectW",last_error})) if last_error==0xe0f00000|p::OBJECT)
            );
            assert!(
                !report
                    .events()
                    .iter()
                    .any(|e| e.operation == p::READ || e.operation == p::CONVERT)
            );
        }
        assert!(actual.cleanup.0.iter().all(Option::is_none));
        assert_eq!(report.after_capture, report.before);
        assert_eq!(source_report.after_capture, source_report.before);
        println!(
            "ScreenDC safety classification: mask={mask:#010x}, bitmap_depth={}, original sample_success={}, Rust sample_success={}; reference tail defined only for instrumented short/layout input",
            report.bitmap_depth,
            source.is_ok(),
            actual.image.is_ok()
        );
        drop(source);
        drop(actual);
        assert_eq!(adapter.image_owners()?, owners);
    }
    window.close()?;
    Ok(())
}
