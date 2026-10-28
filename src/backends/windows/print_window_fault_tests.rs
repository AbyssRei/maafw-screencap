// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::windows::parity_adapter::{print_fault as f, probe as p};

fn native_name(operation: u32) -> &'static str {
    match operation {
        p::GET_DC => "GetDC",
        p::CREATE_DC => "CreateCompatibleDC",
        p::CLIENT_RECT => "GetClientRect",
        p::CREATE_BITMAP => "CreateCompatibleBitmap",
        p::SELECT => "SelectObject",
        p::COPY => "PrintWindow",
        p::READ => "GetDIBits",
        f::READ_DESELECT => "SelectObject.before-GetDIBits",
        f::READ_RESELECT => "SelectObject.after-GetDIBits",
        _ => panic!("unexpected primary operation {operation}"),
    }
}
fn check_native(error: &CaptureError<i32>, operation: u32) {
    match error {
        CaptureError::Native(native) => {
            assert_eq!(native.operation, native_name(operation));
            assert_eq!(native.last_error, 0xe0f00000 | operation);
        }
        _ => panic!("expected native operation {operation}, got {error:?}"),
    }
}
fn check_cleanup(attempt: &Attempt<Image, i32>, report: &p::Report) {
    let mut expected = [[None; 4]; 2];
    let mut capture = 0;
    let mut entered = false;
    for event in report.events() {
        if event.operation == p::CLIENT_RECT {
            if entered {
                capture += 1;
            }
            entered = true;
            assert!(capture < 2);
        }
        let (name, slot) = match event.operation {
            p::RESTORE => ("SelectObject.restore", 0),
            p::DELETE_BITMAP => ("DeleteObject", 1),
            p::DELETE_DC => ("DeleteDC", 2),
            p::RELEASE_DC => ("ReleaseDC", 3),
            _ => continue,
        };
        if event.result == 0 {
            expected[capture][slot] = Some(NativeFailure {
                operation: name,
                last_error: event.last_error,
            });
        }
    }
    assert_eq!(attempt.main_cleanup.0, expected[0]);
    assert_eq!(attempt.popup_cleanup.0, expected[1]);
}

#[test]
#[ignore = "explicit original PrintWindow native failures and two-window cleanup ledger"]
fn matches_source_native_failures_without_losing_main_or_popup_cleanup()
-> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let mut cases = vec![0];
    for target in [0, f::POPUP_ONLY] {
        for operation in [
            p::CLIENT_RECT,
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
            cases.push(target | p::fault(operation));
        }
        cases.push(target | p::fault(p::COPY) | p::fault(p::DELETE_DC));
        cases.push(target | p::fault(p::CONVERT) | p::fault(p::RESTORE) | p::fault(p::DELETE_DC));
    }
    for mask in cases {
        let owners = adapter.image_owners()?;
        let mut fixture = adapter.print_fixture((25, 17), (5, 3), 1)?;
        let baseline = adapter.objects();
        let (source, source_report) = adapter.print_fault_reference(fixture.hwnd(), mask)?;
        source_report.check_fixture();
        let probe = adapter.print_fault_probe(mask)?;
        let actual = PrintWindowCapture::new(fixture.hwnd())
            .capture_with_api(&probe, &mut adapter.converter(0));
        let report = probe.finish()?;
        report.check_fixture();
        assert_eq!(
            report.print_comparable(),
            source_report.print_comparable(),
            "mask={mask:#010x}"
        );
        assert_eq!(report.before, baseline);
        assert_eq!(
            report.after_capture, source_report.after_capture,
            "native residuals before separate fixture recovery"
        );
        check_cleanup(&actual, &report);
        assert_eq!(actual.image.is_ok(), source.is_ok(), "mask={mask:#010x}");
        let popup = mask & f::POPUP_ONLY != 0;
        let primary = [
            p::CLIENT_RECT,
            p::GET_DC,
            p::CREATE_DC,
            p::CREATE_BITMAP,
            p::SELECT,
            p::COPY,
            p::READ,
        ]
        .into_iter()
        .find(|operation| mask & p::fault(*operation) != 0);
        if mask & p::fault(p::CONVERT) != 0 {
            assert_eq!(source.as_ref().err(), Some(&2));
            assert!(matches!(actual.image, Err(CaptureError::Conversion(2))));
            assert!(actual.popup_failure.is_none());
        } else if let Some(operation) = primary {
            if popup {
                assert!(actual.image.is_ok());
                check_native(
                    actual
                        .popup_failure
                        .as_ref()
                        .expect("optional popup failure"),
                    operation,
                );
            } else {
                check_native(
                    actual.image.as_ref().err().expect("main capture failure"),
                    operation,
                );
                assert!(actual.popup_failure.is_none());
            }
        } else {
            assert!(actual.image.is_ok());
            assert!(actual.popup_failure.is_none());
        }
        if let (Ok(image), Ok(expected)) = (&actual.image, &source) {
            let mode = u32::from(!(popup && primary.is_some()));
            assert_pixels(image, (5, 3), (25, 17), mode);
            assert_pixels(expected, (5, 3), (25, 17), mode);
            assert_eq!(image.bytes(), expected.bytes());
        }
        println!(
            "PrintWindow native failure parity: mask={mask:#010x}, events={}, residual delta={} before separate fixture recovery",
            report.count,
            i64::from(report.after_capture) - i64::from(report.before)
        );
        drop(actual);
        drop(source);
        assert_eq!(adapter.image_owners()?, owners);
        assert_eq!(adapter.objects(), baseline);
        fixture.close()?;
    }
    Ok(())
}

#[test]
#[ignore = "explicit classification of PrintWindow native-precondition and short-read safety adaptations"]
fn classifies_read_selection_failures_and_positive_short_scanline_returns()
-> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    for target in [0, f::POPUP_ONLY] {
        for failure in [
            p::SHORT_READ,
            p::fault(f::READ_DESELECT),
            p::fault(f::READ_RESELECT),
        ] {
            let mask = target | failure;
            let owners = adapter.image_owners()?;
            let mut fixture = adapter.print_fixture((25, 17), (5, 3), 1)?;
            let baseline = adapter.objects();
            let (source, source_report) = adapter.print_fault_reference(fixture.hwnd(), mask)?;
            source_report.check_fixture();
            let source = source
                .map_err(|status| format!("source safety observation {mask:#x}: {status}"))?;
            assert_pixels(&source, (5, 3), (25, 17), 1);
            let probe = adapter.print_fault_probe(mask)?;
            let actual = PrintWindowCapture::new(fixture.hwnd())
                .capture_with_api(&probe, &mut adapter.converter(0));
            let report = probe.finish()?;
            report.check_fixture();
            check_cleanup(&actual, &report);
            assert_eq!(report.after_capture, baseline);
            let error = if target == 0 {
                actual
                    .image
                    .as_ref()
                    .err()
                    .expect("reject unsafe main sample")
            } else {
                let image = actual
                    .image
                    .as_ref()
                    .expect("keep main on native popup failure");
                assert_pixels(image, (5, 3), (25, 17), 0);
                actual
                    .popup_failure
                    .as_ref()
                    .expect("retain optional popup failure")
            };
            if failure == p::SHORT_READ {
                assert!(
                    matches!(error, CaptureError::IncompleteScanlines { copied, required }
                    if (*copied, *required) == if target == 0 { (11, 23) } else { (8, 17) })
                );
            } else {
                check_native(
                    error,
                    if failure == p::fault(f::READ_DESELECT) {
                        f::READ_DESELECT
                    } else {
                        f::READ_RESELECT
                    },
                );
            }
            println!(
                "PrintWindow safety classification: mask={mask:#010x}, source success, Rust main_success={}",
                actual.image.is_ok()
            );
            drop(source);
            drop(actual);
            assert_eq!(adapter.image_owners()?, owners);
            fixture.close()?;
        }
    }
    Ok(())
}
