// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::windows::parity_adapter::{Adapter, Image};
use std::error::Error;

#[path = "print_window_read_tests.rs"]
mod read_boundary;

#[test]
#[ignore = "explicit original PrintWindow query/fallback parity"]
fn preserves_root_popup_eligibility_and_origin_failure_fallbacks() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    for scenario in 0..8 {
        let owners = adapter.image_owners()?;
        let mut fixture = adapter.print_fixture((25, 17), (5, 3), 1)?;
        let baseline = adapter.objects();
        let (expected, expected_queries) =
            adapter.print_query_reference(fixture.hwnd(), scenario)?;
        let probe = adapter.print_query_probe(fixture.hwnd(), scenario)?;
        let mut converter = Observed {
            adapter: &adapter,
            baseline,
            conversions: 0,
            overlays: 0,
            fail_at: 0,
        };
        let actual =
            PrintWindowCapture::new(fixture.hwnd()).capture_with_api(&probe, &mut converter);
        let queries = probe.finish()?;
        assert_eq!(
            queries.events(),
            expected_queries.events(),
            "scenario={scenario}"
        );
        assert!(actual.main_cleanup.0.iter().all(Option::is_none));
        assert!(actual.popup_cleanup.0.iter().all(Option::is_none));
        assert_eq!(adapter.objects(), baseline);
        let expected = expected.map_err(|status| format!("query source {scenario}: {status}"))?;
        let image = actual
            .image
            .map_err(|error| format!("query Rust {scenario}: {error:?}"))?;
        let composed = u32::from(scenario <= 1);
        assert_pixels(&expected, (5, 3), (25, 17), composed);
        assert_pixels(&image, (5, 3), (25, 17), composed);
        assert_eq!(image.bytes(), expected.bytes(), "scenario={scenario}");
        assert_eq!(converter.overlays, usize::try_from(composed)?);
        assert_eq!(
            converter.conversions,
            if (2..=5).contains(&scenario) { 1 } else { 2 }
        );
        let origin_queries: Vec<_> = queries
            .events()
            .iter()
            .filter(|event| event.operation == 5)
            .collect();
        match scenario {
            6 | 7 => {
                assert_eq!(origin_queries.len(), usize::try_from(scenario - 5)?);
                assert!(
                    matches!(actual.popup_failure, Some(CaptureError::Native(NativeFailure {
                    operation: "ClientToScreen", last_error
                })) if last_error == 0xe0f10000 | scenario)
                );
            }
            _ => {
                assert!(actual.popup_failure.is_none());
                assert_eq!(origin_queries.len(), if composed == 1 { 2 } else { 0 });
            }
        }
        fixture.close()?;
        drop(image);
        drop(expected);
        assert_eq!(adapter.image_owners()?, owners);
        println!(
            "PrintWindow query parity: scenario={scenario}, {} native query events",
            queries.events().len()
        );
    }
    Ok(())
}

#[test]
#[ignore = "explicit compiled OpenCV source geometry boundary parity"]
fn preserves_opencv_intersection_at_extreme_representable_offsets() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let offsets = [
        i64::from(i32::MIN),
        -25,
        -24,
        -1,
        0,
        1,
        36,
        37,
        i64::from(i32::MAX) - 1,
        i64::from(i32::MAX),
    ];
    for canvas in [(37, 23), (i32::MAX, i32::MAX)] {
        for popup in [(25, 17), (i32::MAX, i32::MAX)] {
            for x in offsets {
                for y in offsets {
                    assert_eq!(
                        intersection::<()>(canvas, popup, (x, y)).unwrap(),
                        adapter.print_intersection(canvas, popup, (x, y))?,
                        "canvas={canvas:?}, popup={popup:?}, offset=({x},{y})"
                    );
                }
            }
        }
    }
    for offset in [(i64::from(i32::MIN) - 1, 0), (0, i64::from(i32::MAX) + 1)] {
        assert!(intersection::<()>((37, 23), (25, 17), offset).is_err());
        assert!(
            adapter
                .print_intersection((37, 23), (25, 17), offset)
                .is_err()
        );
    }
    println!("PrintWindow geometry: 400 compiled OpenCV/Rust boundary combinations");
    Ok(())
}

struct Observed<'a> {
    adapter: &'a Adapter,
    baseline: u32,
    conversions: usize,
    overlays: usize,
    fail_at: usize,
}
impl BgraConverter for Observed<'_> {
    type Image = Image;
    type Error = i32;
    fn bgra_to_bgr(&mut self, input: BgraView<'_>) -> Result<Image, i32> {
        self.conversions += 1;
        assert_eq!(
            self.adapter.objects(),
            self.baseline + 3,
            "conversion remains before native cleanup"
        );
        self.adapter
            .converter(u32::from(self.conversions == self.fail_at))
            .bgra_to_bgr(input)
    }
}
impl PrintConverter for Observed<'_> {
    fn overlay(
        &mut self,
        destination: &mut Image,
        source: &Image,
        rectangle: Overlay,
    ) -> Result<(), i32> {
        self.overlays += 1;
        assert_eq!(
            self.adapter.objects(),
            self.baseline,
            "both source captures release native resources before BGR composition"
        );
        self.adapter
            .converter(0)
            .overlay(destination, source, rectangle)
    }
}

fn expected_color(x: usize, y: usize, width: usize, height: usize, popup: bool) -> [u8; 3] {
    let palette = if popup {
        [[0xe3, 0x42, 0x19], [0x57, 0x8b, 0xc1], [0x9d, 0x23, 0xf5]]
    } else {
        [[0x1f, 0x53, 0xa7], [0xb9, 0x4d, 0x27], [0x3b, 0xe2, 0x81]]
    };
    palette[if x < width / 3 {
        0
    } else if y < height / 2 {
        1
    } else {
        2
    }]
}
fn assert_pixels(image: &Image, offset: (i32, i32), popup: (i32, i32), mode: u32) {
    assert_eq!(image.dimensions(), (37, 23));
    for y in 0..23 {
        for x in 0..37 {
            let sx = x - offset.0;
            let sy = y - offset.1;
            let in_popup = mode == 1 && sx >= 0 && sy >= 0 && sx < popup.0 && sy < popup.1;
            let color = if in_popup {
                expected_color(
                    usize::try_from(sx).unwrap(),
                    usize::try_from(sy).unwrap(),
                    usize::try_from(popup.0).unwrap(),
                    usize::try_from(popup.1).unwrap(),
                    true,
                )
            } else {
                expected_color(
                    usize::try_from(x).unwrap(),
                    usize::try_from(y).unwrap(),
                    37,
                    23,
                    false,
                )
            };
            let index = usize::try_from(y)
                .unwrap()
                .checked_mul(image.stride())
                .unwrap()
                .checked_add(usize::try_from(x).unwrap().checked_mul(3).unwrap())
                .unwrap();
            assert_eq!(
                &image.bytes()[index..index + 3],
                &color,
                "pixel ({x},{y}), offset={offset:?}, mode={mode}"
            );
        }
    }
}

#[test]
#[ignore = "explicit original PrintWindow/owned-rendering parity"]
fn matches_original_popup_composition_and_retains_pixels_after_fixture_teardown()
-> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    for (offset, popup, mode) in [
        ((5, 3), (25, 17), 1),
        ((-7, -5), (25, 17), 1),
        ((30, 20), (25, 17), 1),
        ((37, 0), (25, 17), 1),
        ((-25, 0), (25, 17), 1),
        ((5, 3), (25, 17), 2),
        ((0, 0), (25, 17), 0),
        ((5, 3), (0, 17), 1),
    ] {
        let owners = adapter.image_owners()?;
        let all_resources = adapter.objects();
        let mut retained: Option<Image> = None;
        for _ in 0..20 {
            let mut fixture = adapter.print_fixture(popup, offset, mode)?;
            let baseline = adapter.objects();
            let before_render = fixture.stats()?;
            let original = adapter
                .print_reference(fixture.hwnd())
                .map_err(|v| format!("original: {v}"))?;
            let mut converter = Observed {
                adapter: &adapter,
                baseline,
                conversions: 0,
                overlays: 0,
                fail_at: 0,
            };
            let result = PrintWindowCapture::new(fixture.hwnd()).capture(&mut converter);
            assert!(result.main_cleanup.0.iter().all(Option::is_none));
            assert!(result.popup_cleanup.0.iter().all(Option::is_none));
            let image = result
                .image
                .map_err(|e| format!("Rust PrintWindow: {e:?}"))?;
            assert_eq!(adapter.objects(), baseline);
            assert_pixels(&image, offset, popup, mode);
            if let Some(index) = image
                .bytes()
                .iter()
                .zip(original.bytes())
                .position(|(a, b)| a != b)
            {
                panic!(
                    "source/Rust pixel difference: offset={offset:?}, popup={popup:?}, mode={mode}, byte={index}, Rust={}, original={}",
                    image.bytes()[index],
                    original.bytes()[index]
                );
            }
            assert_eq!(image.bytes().len(), original.bytes().len());
            let stats = fixture.stats()?;
            assert_eq!(stats[4], 0, "owned rendering must succeed");
            // PW_RENDERFULLCONTENT uses WM_PAINT rather than WM_PRINTCLIENT
            // on this host. Observe actual owned rendering without pretending
            // that PrintWindow guarantees one particular message path.
            assert!(stats[0] > 0);
            assert!(
                stats[0].checked_add(stats[1]).unwrap()
                    >= before_render[0]
                        .checked_add(before_render[1])
                        .unwrap()
                        .checked_add(2)
                        .unwrap(),
                "both actual captures render the main fixture: before={before_render:?}, after={stats:?}"
            );
            if mode == 1 && popup.0 > 0 {
                assert!(
                    stats[2].checked_add(stats[3]).unwrap()
                        >= before_render[2]
                            .checked_add(before_render[3])
                            .unwrap()
                            .checked_add(2)
                            .unwrap(),
                    "both actual captures render the popup fixture: before={before_render:?}, after={stats:?}"
                );
            }
            drop(original);
            fixture.close()?;
            assert_eq!(
                adapter.objects(),
                all_resources,
                "window/class/DC/bitmap lifetimes"
            );
            assert_pixels(&image, offset, popup, mode);
            if retained.is_none() {
                retained = Some(image);
            } else {
                drop(image);
            }
            assert_eq!(adapter.image_owners()?, owners + 1);
        }
        assert_pixels(retained.as_ref().unwrap(), offset, popup, mode);
        drop(retained);
        assert_eq!(adapter.image_owners()?, owners);
        println!(
            "PrintWindow parity: offset={offset:?}, popup={popup:?}, mode={mode}, 20 fixture/capture lifecycles"
        );
    }
    Ok(())
}

#[test]
#[ignore = "explicit PrintWindow failure/exception ownership"]
fn distinguishes_optional_popup_failure_from_conversion_exception() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let mut fixture = adapter.print_fixture((25, 17), (5, 3), 1)?;
    let baseline = adapter.objects();
    let owners = adapter.image_owners()?;
    for fail_at in [1, 2] {
        let source = adapter.print_conversion_reference(fixture.hwnd(), fail_at)?;
        assert_eq!(
            source.image.err(),
            Some(2),
            "source conversion must escape popup fallback"
        );
        assert_eq!(source.calls, u64::from(fail_at));
        assert_eq!(source.baseline, u64::from(baseline));
        assert_eq!(source.at_conversion[0], u64::from(baseline + 3));
        assert_eq!(
            source.at_conversion[1],
            if fail_at == 2 {
                u64::from(baseline + 3)
            } else {
                0
            }
        );
        assert_eq!(
            source.after_capture,
            u64::from(baseline),
            "source must unwind its GDI resources"
        );
        assert_eq!(
            source.image_owners, owners,
            "source must not export an image after a conversion exception"
        );
        assert_eq!(source.overflow, 0);
        let bgr_allocations = u64::from(fail_at == 2);
        assert_eq!(source.bgr_allocations, bgr_allocations);
        assert_eq!(source.bgr_releases, bgr_allocations);
        assert_eq!(source.bgr_bytes, bgr_allocations * 37 * 23 * 3);
        assert_eq!(source.bgr_freed_bytes, source.bgr_bytes);
        assert_eq!(
            source.allocator_errors, 0,
            "source BGR output must release before the probe ends"
        );
        let mut converter = Observed {
            adapter: &adapter,
            baseline,
            conversions: 0,
            overlays: 0,
            fail_at: usize::try_from(fail_at)?,
        };
        let result = PrintWindowCapture::new(fixture.hwnd()).capture(&mut converter);
        assert!(matches!(result.image, Err(CaptureError::Conversion(2))));
        assert!(
            result.popup_failure.is_none(),
            "conversion exceptions must abort the whole source operation"
        );
        assert_eq!(converter.conversions, usize::try_from(fail_at)?);
        assert_eq!(converter.overlays, 0);
        assert_eq!(adapter.objects(), baseline);
        assert_eq!(
            adapter.image_owners()?,
            owners,
            "main image drops if popup conversion throws"
        );
        println!(
            "PrintWindow conversion exception parity: fail_at={fail_at}, GDI cleanup/exported owners balanced, source BGR allocations={}, releases={}, bytes={}, freed_bytes={}",
            source.bgr_allocations, source.bgr_releases, source.bgr_bytes, source.bgr_freed_bytes
        );
    }
    let hwnd = fixture.hwnd();
    fixture.close()?;
    for hwnd in [std::ptr::null_mut(), hwnd] {
        assert!(adapter.print_reference(hwnd).is_err());
        let result = PrintWindowCapture::new(hwnd).capture(&mut adapter.converter(0));
        assert!(result.image.is_err());
        assert!(result.main_cleanup.0.iter().all(Option::is_none));
        assert!(result.popup_cleanup.0.iter().all(Option::is_none));
    }
    let mut main_only = adapter.print_fixture((25, 17), (5, 3), 0)?;
    let main_baseline = adapter.objects();
    let unreached = adapter.print_conversion_reference(main_only.hwnd(), 2)?;
    assert_eq!(
        unreached.image.err(),
        Some(4),
        "an unreached injected fault must not export the observed Mat"
    );
    assert_eq!(unreached.calls, 1);
    assert_eq!(unreached.after_capture, u64::from(main_baseline));
    assert_eq!(unreached.image_owners, owners);
    assert_eq!((unreached.bgr_allocations, unreached.bgr_releases), (1, 1));
    assert_eq!(
        (unreached.bgr_bytes, unreached.bgr_freed_bytes),
        (2553, 2553)
    );
    assert_eq!(unreached.allocator_errors, 0);
    main_only.close()?;
    Ok(())
}
