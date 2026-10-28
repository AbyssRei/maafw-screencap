// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#![cfg(feature = "prototype-windows-com")]
use super::*;
use crate::backends::windows::{frame_pool, parity_adapter::Adapter};
use crate::platform::windows::wgc;
use std::{error::Error, sync::atomic::Ordering};

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
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: Owns exactly one successful initialization on this caller.
        unsafe { host::RoUninitialize() };
    }
}

fn compare_state(actual: [u64; 6], expected: [u64; 6], label: &str) {
    println!("{label}: actual={actual:?}, source={expected:?}");
    // Foreground authority differs between isolated processes. Keep that bit
    // as evidence; explicit minimized fixtures independently require nonforeground
    // below so the monitor does not immediately restore their pseudo state.
    for field in [0, 1, 2, 4, 5] {
        assert_eq!(actual[field], expected[field], "{label} field={field}");
    }
}

#[test]
#[ignore = "explicit original FramePool wrapper images/states and native rendering adapter"]
fn preserves_original_pseudo_wrapper_pixels_and_stop_inactive_states() -> Result<(), Box<dyn Error>>
{
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_FRAMEPOOL_PSEUDO_REFERENCE_ROOT")
            .ok_or("original FramePool wrapper records required")?,
    );
    if !root.is_absolute() {
        return Err("original wrapper record root must be absolute".into());
    }
    let cases = [
        "pseudo-visible",
        "pseudo-stop",
        "pseudo-inactive",
        "pseudo-layered-stop",
        "pseudo-layered-inactive",
    ];
    let mut references = Vec::new();
    for case in cases {
        let pixels = std::fs::read(root.join(case).join("source.bgr"))?;
        assert_eq!(pixels.len(), 96 * 64 * 3);
        let text = std::fs::read_to_string(root.join(case).join("window-states.txt"))?;
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 4);
        let mut states = [[0_u64; 6]; 4];
        for (row, label) in ["before", "captured", "inactive", "closed"]
            .iter()
            .enumerate()
        {
            let words: Vec<_> = lines[row].split_ascii_whitespace().collect();
            assert_eq!(words.len(), 7);
            assert_eq!(words[0], *label);
            for column in 0..6 {
                states[row][column] = words[column + 1].parse()?;
            }
        }
        references.push((pixels, states));
    }
    // SAFETY: The native wrapper and its inner remain on this initialized caller;
    // the helper's monitor uses only its original Win32 observation operations.
    unsafe { host::RoInitialize(host::RO_INIT_MULTITHREADED).ok()? };
    let apartment = Apartment;
    let dpi = unsafe { host::GetThreadDpiAwarenessContext() };
    let adapter = Adapter::load()?;
    let baseline = adapter.image_owners()?;
    let close_errors = frame_pool::DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire);
    let mut retained = Vec::new();
    for cycle in 0..20 {
        let case_index = cycle % cases.len();
        let case = cases[case_index];
        let (expected, states) = &references[case_index];
        let minimized = case != "pseudo-visible";
        let layered = case.contains("layered");
        let inactive = case.ends_with("inactive");
        let mut fixture = adapter.render_fixture(96, 64)?;
        if minimized {
            fixture.prepare_pseudo(layered)?;
        }
        let before = fixture.pseudo_state()?;
        compare_state(before, states[0], &format!("cycle={cycle} {case} before"));
        assert_eq!(before[2], u64::from(minimized));
        if minimized {
            assert_eq!(before[3], 0);
            assert_eq!(before[1], if layered { 73 } else { 255 });
        }
        let mut capture = FramePoolWithPseudoMinimize::new(fixture.hwnd())
            .map_err(|error| format!("wrapper start: {error:?}"))?;
        let mut converter = adapter.converter(0);
        let attempt = capture.capture(&mut converter);
        assert!(attempt.evidence.cleanup.iter().all(Option::is_none));
        assert!(!attempt.evidence.replayed);
        let image = attempt
            .image
            .map_err(|error| format!("cycle={cycle} {case}: {error:?}"))?;
        assert_eq!(image.dimensions(), (96, 64));
        assert_eq!(image.bytes(), expected);
        assert_eq!((converter.conversions, converter.clones), (1, 1));
        let captured = fixture.pseudo_state()?;
        compare_state(
            captured,
            states[1],
            &format!("cycle={cycle} {case} captured"),
        );
        assert_eq!(captured[2], 0);
        if minimized {
            assert_eq!(captured[3], 0);
            assert_eq!(captured[0], before[0] | 0x80000 | 0x20);
            assert_eq!(captured[1], 0);
            assert!(capture.helper.as_ref().unwrap().is_pseudo_minimized());
        }
        if inactive {
            capture.inactive();
            assert!(!capture.helper.as_ref().unwrap().is_pseudo_minimized());
        }
        compare_state(
            fixture.pseudo_state()?,
            states[2],
            &format!("cycle={cycle} {case} inactive"),
        );
        let helper = capture.helper.as_ref().unwrap().evidence();
        assert_eq!(helper.failures, 0);
        assert!(!helper.worker_panicked);
        drop(capture); // Helper fully stops/joins/restores before inner closes.
        let closed = fixture.pseudo_state()?;
        compare_state(closed, states[3], &format!("cycle={cycle} {case} closed"));
        assert_eq!(closed[0], before[0]);
        assert_eq!(closed[1], before[1]);
        assert_eq!(closed[2], u64::from(minimized && !inactive));
        assert_eq!(
            frame_pool::DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire),
            close_errors
        );
        assert_eq!(
            adapter.image_owners()?,
            baseline + u64::try_from(cycle)? + 1
        );
        assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
        assert_eq!(wgc::EVENT_CLOSE_ERROR.load(Ordering::Acquire), 0);
        fixture.close()?;
        retained.push((image, expected));
    }
    drop(adapter);
    assert!(
        unsafe { host::AreDpiAwarenessContextsEqual(dpi, host::GetThreadDpiAwarenessContext()) }
            .as_bool()
    );
    drop(apartment);
    for (image, expected) in retained {
        assert_eq!(image.bytes(), expected);
    }
    Ok(())
}
