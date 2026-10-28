// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::windows::print_window_pseudo::PrintWindowWithPseudoMinimize;
use crate::platform::windows::pseudo_minimize::Helper;
use std::time::{Duration, Instant};

#[test]
#[ignore = "explicit original/Rust pseudo-minimized PrintWindow capture and native worker teardown"]
fn captures_pseudo_minimized_windows_and_preserves_stop_and_inactive_effects()
-> Result<(), Box<dyn Error>> {
    run_lifecycles(false)
}

#[test]
#[ignore = "explicit layered original/Rust pseudo-minimized PrintWindow capture"]
fn captures_layered_pseudo_minimized_windows_and_restores_original_alpha()
-> Result<(), Box<dyn Error>> {
    run_lifecycles(true)
}

fn run_lifecycles(layered: bool) -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let owners = adapter.image_owners()?;
    for inactive in [false, true] {
        for cycle in 0..20 {
            let mut source_fixture = adapter.print_fixture((0, 0), (0, 0), 0)?;
            source_fixture.prepare_pseudo(layered)?;
            let source_initial = adapter.pseudo_state(source_fixture.hwnd())?;
            assert_eq!(source_initial[2..4], [1, 0]);
            let mut source = adapter.pseudo_original(source_fixture.hwnd())?;
            let expected = source.capture()?;
            let source_pseudo = adapter.pseudo_state(source_fixture.hwnd())?;
            assert_eq!(source_pseudo[1..], [0, 0, 0, 37, 23]);
            assert_eq!(source_pseudo[0] & 0x80020, 0x80020);
            if inactive {
                source.inactive()?;
            }
            source.close()?;
            let source_final = adapter.pseudo_state(source_fixture.hwnd())?;
            assert_eq!(source_final[0..2], source_initial[0..2]);
            assert_eq!(source_final[2], u64::from(!inactive));
            source_fixture.close()?;

            let mut fixture = adapter.print_fixture((0, 0), (0, 0), 0)?;
            fixture.prepare_pseudo(layered)?;
            let initial = adapter.pseudo_state(fixture.hwnd())?;
            assert_eq!(initial, source_initial);
            let baseline = adapter.objects();
            let mut capture = PrintWindowWithPseudoMinimize::new(fixture.hwnd())
                .map_err(|error| format!("Rust pseudo start: {error:?}"))?;
            let mut converter = Observed {
                adapter: &adapter,
                baseline,
                conversions: 0,
                overlays: 0,
                fail_at: 0,
            };
            let attempt = capture.capture(&mut converter);
            assert!(attempt.main_cleanup.0.iter().all(Option::is_none));
            assert!(attempt.popup_failure.is_none());
            let image = attempt
                .image
                .map_err(|error| format!("Rust pseudo capture: {error:?}"))?;
            println!(
                "Pseudo capture pixels: layered={layered}, inactive={inactive}, cycle={cycle}, original00={:?}, Rust00={:?}, unequal_bytes={}",
                &expected.bytes()[..3],
                &image.bytes()[..3],
                expected
                    .bytes()
                    .iter()
                    .zip(image.bytes())
                    .filter(|(left, right)| left != right)
                    .count()
            );
            assert_eq!(image.bytes(), expected.bytes());
            assert_pixels(&expected, (0, 0), (0, 0), 0);
            assert_pixels(&image, (0, 0), (0, 0), 0);
            assert_eq!(adapter.pseudo_state(fixture.hwnd())?, source_pseudo);
            assert_eq!(converter.conversions, 1);
            assert_eq!(converter.overlays, 0);
            if inactive {
                capture.inactive();
            }
            drop(capture); // actual monitor join, before native restoration assertion
            assert_eq!(adapter.pseudo_state(fixture.hwnd())?, source_final);
            assert_eq!(adapter.objects(), baseline);
            fixture.close()?;
            assert_pixels(&image, (0, 0), (0, 0), 0);
            assert_pixels(&expected, (0, 0), (0, 0), 0);
            drop(image);
            drop(expected);
            assert_eq!(adapter.image_owners()?, owners);
            println!(
                "Pseudo PrintWindow native parity: layered={layered}, inactive={inactive}, cycle={cycle}"
            );
        }
    }
    Ok(())
}

fn wait_for_monitor(
    mut condition: impl FnMut() -> Result<bool, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition()? {
        if Instant::now() >= deadline {
            return Err("native helper monitor did not observe the transition".into());
        }
        // Observe the original 100ms monitor. This never retries a capture,
        // changes its output, or inserts waiting into the product helper.
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

#[test]
#[ignore = "explicit original/Rust real monitor foreground parity; requires native foreground control"]
fn restores_original_attributes_after_real_foreground_reactivation() -> Result<(), Box<dyn Error>> {
    run_monitor_cases(false)
}

#[test]
#[ignore = "explicit original/Rust real monitor target-destruction parity"]
fn clears_pseudo_state_after_target_destruction_without_restarting_the_monitor()
-> Result<(), Box<dyn Error>> {
    run_monitor_cases(true)
}

fn run_monitor_cases(target_loss: bool) -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let owners = adapter.image_owners()?;
    for layered in [false, true] {
        for cycle in 0..20 {
            let mut source_fixture = adapter.print_fixture((0, 0), (0, 0), 0)?;
            source_fixture.prepare_pseudo(layered)?;
            let original_state = adapter.pseudo_state(source_fixture.hwnd())?;
            let mut source = adapter.pseudo_helper_original(source_fixture.hwnd())?;
            source.ensure()?;
            assert!(source.is_pseudo()?);
            let expected = adapter
                .print_reference(source_fixture.hwnd())
                .map_err(|status| format!("source helper capture: {status}"))?;
            assert_pixels(&expected, (0, 0), (0, 0), 0);
            if target_loss {
                source_fixture.close()?;
            } else {
                source_fixture.activate()?;
            }
            wait_for_monitor(|| Ok(!source.is_pseudo()?))?;
            source.close()?;
            let final_state = if target_loss {
                None
            } else {
                let state = adapter.pseudo_state(source_fixture.hwnd())?;
                assert_eq!(&state[..2], &original_state[..2]);
                assert_eq!(state[2..], [0, 1, 37, 23]);
                source_fixture.close()?;
                Some(state)
            };

            let mut fixture = adapter.print_fixture((0, 0), (0, 0), 0)?;
            fixture.prepare_pseudo(layered)?;
            let initial = adapter.pseudo_state(fixture.hwnd())?;
            assert_eq!(initial, original_state);
            let baseline = adapter.objects();
            let mut helper = Helper::new(fixture.hwnd())?;
            helper.start()?;
            helper.ensure_not_minimized();
            assert!(helper.is_pseudo_minimized());
            let mut converter = Observed {
                adapter: &adapter,
                baseline,
                conversions: 0,
                overlays: 0,
                fail_at: 0,
            };
            let attempt = PrintWindowCapture::new(fixture.hwnd()).capture(&mut converter);
            assert!(attempt.main_cleanup.0.iter().all(Option::is_none));
            let image = attempt
                .image
                .map_err(|error| format!("Rust helper capture: {error:?}"))?;
            assert_eq!(image.bytes(), expected.bytes());
            assert_pixels(&image, (0, 0), (0, 0), 0);
            if target_loss {
                fixture.close()?;
            } else {
                fixture.activate()?;
            }
            wait_for_monitor(|| Ok(!helper.is_pseudo_minimized()))?;
            helper.stop();
            assert_eq!(helper.evidence(), Default::default());
            if let Some(state) = final_state {
                assert_eq!(adapter.pseudo_state(fixture.hwnd())?, state);
                fixture.close()?;
            }
            assert_pixels(&expected, (0, 0), (0, 0), 0);
            assert_pixels(&image, (0, 0), (0, 0), 0);
            drop(expected);
            drop(image);
            assert_eq!(adapter.image_owners()?, owners);
            println!(
                "Pseudo native monitor parity: layered={layered}, target_loss={target_loss}, cycle={cycle}"
            );
        }
    }
    Ok(())
}
