// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
#[cfg(feature = "prototype-windows")]
#[path = "screen_dc_fault_tests.rs"]
mod native_failures;
#[cfg(feature = "prototype-windows")]
#[path = "screen_dc_geometry_tests.rs"]
mod native_geometry;

#[cfg(feature = "prototype-windows")]
#[test]
#[ignore = "explicit source-only visibility diagnostic, not capture acceptance"]
fn observes_point_window_roots_around_original_screen_samples()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::parity_adapter::Adapter;
    let adapter = Adapter::load()?;
    for cycle in 0..3 {
        let mut fixture = adapter.screen_fixture()?;
        let before = adapter.screen_window_observation(fixture.hwnd())?;
        let image = adapter
            .screen_reference(fixture.hwnd())
            .map_err(|s| format!("source {s}"))?;
        let after = adapter.screen_window_observation(fixture.hwnd())?;
        let pixel = first_nonfixture_pixel(&image);
        println!(
            "source-only visibility cycle {cycle}: before={before:?}, after={after:?}, nonfixture_pixel={pixel:?}"
        );
        record_image(&format!("visibility-{cycle}-source"), &image)?;
        assert_eq!(before[0], 1);
        assert_eq!(before[1], 1);
        assert_eq!(before[2], 0);
        // Point/root and pixel observations are retained without converting
        // this diagnosis into a passing visible-pixel acceptance claim.
        fixture.close()?;
    }
    Ok(())
}
#[path = "screen_dc_synthetic.rs"]
mod synthetic;

#[cfg(feature = "prototype-windows")]
#[test]
#[ignore = "explicit native rendering owner/module lifetime; no desktop pixel capture"]
fn retains_rendering_callback_module_after_adapter_drop_until_joined_close()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::parity_adapter::Adapter;
    let adapter = Adapter::load()?;
    let mut fixture = adapter.screen_fixture()?;
    let owner = fixture.thread_info()?;
    assert_ne!(owner[0], owner[1]);
    assert_eq!(owner[0], owner[2]);
    drop(adapter);
    let stats = fixture.stats()?;
    assert!(stats[0] > 0);
    assert_eq!((stats[1], stats[4]), (0, 0));
    fixture.close()?;
    fixture.close()?; // Empty guard is harmless, not a repeated native close.
    Ok(())
}

#[cfg(feature = "prototype-windows")]
fn first_nonfixture_pixel(
    image: &crate::backends::windows::parity_adapter::Image,
) -> Option<(i32, i32, [u8; 3])> {
    let (width, height) = image.dimensions();
    for y in 0..height {
        for x in 0..width {
            let expected = if x < width / 3 {
                [0x1f, 0x53, 0xa7]
            } else if y < height / 2 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            let offset = usize::try_from(y)
                .unwrap()
                .checked_mul(image.stride())
                .unwrap()
                .checked_add(usize::try_from(x).unwrap().checked_mul(3).unwrap())
                .unwrap();
            let pixel: [u8; 3] = image.bytes()[offset..offset + 3].try_into().unwrap();
            if pixel != expected {
                return Some((x, y, pixel));
            }
        }
    }
    None
}
#[cfg(feature = "prototype-windows")]
fn record_image(
    name: &str,
    image: &crate::backends::windows::parity_adapter::Image,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("MFS_SCREEN_EVIDENCE") {
        let root = std::path::PathBuf::from(path);
        if !root.is_absolute() {
            return Err("MFS_SCREEN_EVIDENCE must be absolute".into());
        }
        std::fs::write(root.join(format!("{name}.bgr")), image.bytes())?;
        std::fs::write(
            root.join(format!("{name}.txt")),
            format!(
                "dimensions={:?}\nstride={}\nfirst_nonfixture_pixel={:?}\n",
                image.dimensions(),
                image.stride(),
                first_nonfixture_pixel(image)
            ),
        )?;
    }
    Ok(())
}

#[cfg(feature = "prototype-windows")]
#[test]
#[ignore = "explicit source-only presentation control; no Rust capture or retry"]
fn observes_original_screen_pixels_before_and_after_fixture_presentation_barrier()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::parity_adapter::Adapter;
    let adapter = Adapter::load()?;
    let mut transitions = 0;
    for cycle in 0..20 {
        let mut window = adapter.screen_fixture()?;
        let visible_a = adapter.screen_window_observation(window.hwnd())?;
        let before = adapter
            .screen_reference(window.hwnd())
            .map_err(|s| format!("source A {s}"))?;
        let barrier = adapter.screen_present()?;
        let after = adapter
            .screen_reference(window.hwnd())
            .map_err(|s| format!("source B {s}"))?;
        let visible_b = adapter.screen_window_observation(window.hwnd())?;
        record_image(&format!("source-control-{cycle}-before"), &before)?;
        record_image(&format!("source-control-{cycle}-after"), &after)?;
        let a = first_nonfixture_pixel(&before);
        let b = first_nonfixture_pixel(&after);
        println!(
            "source-only presentation cycle {cycle}: before={a:?}, barrier={barrier:?}, after={b:?}, visibility_before={visible_a:?}, visibility_after={visible_b:?}"
        );
        assert_eq!(barrier, [1, 0], "fixture submission/presentation evidence");
        assert_eq!(
            b, None,
            "source B after fixture presentation, cycle {cycle}"
        );
        transitions += usize::from(a.is_some());
        window.close().map_err(|s| format!("close {s}"))?;
    }
    println!(
        "source-only nonfixture->fixture transitions: {transitions}/20; original A outcomes retained, never counted as acceptance"
    );
    Ok(())
}

#[cfg(feature = "prototype-windows")]
#[test]
#[ignore = "explicit fixture-only native resource control, no capture"]
fn isolates_builtin_window_initialization_from_capture_resource_counts()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::parity_adapter::Adapter;
    let adapter = Adapter::load()?;
    let before = adapter.objects();
    for cycle in 0..3 {
        let mut window = adapter.window(37, 23).map_err(|s| format!("fixture {s}"))?;
        let open = adapter.objects();
        window.close().map_err(|s| format!("fixture close {s}"))?;
        let closed = adapter.objects();
        println!("fixture-only cycle {cycle}: initial={before}, open={open}, closed={closed}");
        assert_eq!(closed, open, "built-in fixture resource control");
    }
    Ok(())
}

#[cfg(feature = "prototype-windows")]
#[test]
#[ignore = "explicit visible desktop/reference parity; requires ScreenDC parity adapter"]
fn compares_original_visible_client_pixels_and_retains_cpu_images_after_close()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::parity_adapter::{Adapter, Image};
    fn known_pixels(image: &Image) {
        let (width, height) = image.dimensions();
        for y in 0..height {
            for x in 0..width {
                let expected = if x < width / 3 {
                    [0x1f, 0x53, 0xa7]
                } else if y < height / 2 {
                    [0xb9, 0x4d, 0x27]
                } else {
                    [0x3b, 0xe2, 0x81]
                };
                let offset = usize::try_from(y)
                    .unwrap()
                    .checked_mul(image.stride())
                    .unwrap()
                    .checked_add(usize::try_from(x).unwrap().checked_mul(3).unwrap())
                    .unwrap();
                assert_eq!(
                    &image.bytes()[offset..offset + 3],
                    &expected,
                    "visible pixel ({x},{y})"
                );
            }
        }
    }
    let adapter = Adapter::load()?;
    for cycle in 0..20 {
        let mut window = adapter.screen_fixture()?;
        // Fixture-only cold-process control shows CreateWindow initializes
        // nine process GDI objects, retained after its first DestroyWindow.
        // Capture accounting begins after fixture creation, with both source
        // and Rust measured against the same initialized process state.
        let baseline = adapter.objects();
        // An unpumped STATIC fixture also failed after this barrier in the
        // formal batch. This fixture now owns a real UI loop and redraws on
        // WM_PAINT; presentation is still established once, never retried.
        assert_eq!(adapter.screen_present()?, [1, 0]);
        let stats = window.stats()?;
        assert!(stats[0] > 0, "owner must have handled actual WM_PAINT");
        assert_eq!(
            (stats[1], stats[4]),
            (0, 0),
            "no PrintWindow path or rendering error"
        );
        println!(
            "ScreenDC fixture owner={:?}, paint evidence={stats:?}",
            window.thread_info()?
        );
        let mut capture = ScreenDcCapture::new(window.hwnd());
        let before_source = adapter.screen_window_observation(window.hwnd())?;
        let reference = adapter
            .screen_reference(window.hwnd())
            .map_err(|s| format!("source capture status {s}, cycle {cycle}"))?;
        let after_source = adapter.screen_window_observation(window.hwnd())?;
        assert_eq!(adapter.objects(), baseline, "original ScreenDC cleanup");
        let mut converter = adapter.converter(0);
        let attempt = capture.capture(&mut converter);
        let after_rust = adapter.screen_window_observation(window.hwnd())?;
        println!(
            "ScreenDC cycle {cycle} visibility: before_source={before_source:?}, after_source={after_source:?}, after_rust={after_rust:?}"
        );
        assert!(
            attempt.cleanup.0.iter().all(Option::is_none),
            "{:?}",
            attempt.cleanup
        );
        let coordination = attempt.coordination.unwrap();
        println!(
            "ScreenDC cycle {cycle}: foreground={}, geometry={:?}",
            coordination.foreground, coordination.geometry
        );
        let image = attempt.image.map_err(|e| format!("Rust ScreenDC {e:?}"))?;
        assert_eq!(image.dimensions(), reference.dimensions());
        record_image(&format!("parity-{cycle}-source"), &reference)?;
        record_image(&format!("parity-{cycle}-rust"), &image)?;
        let first_difference = image
            .bytes()
            .iter()
            .zip(reference.bytes())
            .position(|(a, b)| a != b);
        assert_eq!(
            first_difference, None,
            "source/Rust visible pixels, cycle {cycle}"
        );
        known_pixels(&reference);
        known_pixels(&image);
        assert_eq!(adapter.objects(), baseline, "Rust ScreenDC cleanup");
        let stats = window.stats()?;
        assert_eq!((stats[1], stats[4]), (0, 0));
        assert!(converter.observed_objects.unwrap() >= baseline.checked_add(2).unwrap());
        window.close().map_err(|s| format!("close status {s}"))?;
        assert_eq!(adapter.objects(), baseline);
        known_pixels(&reference);
        known_pixels(&image);
    }
    Ok(())
}
