// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use std::fmt::Write;

fn report() -> String {
    let windows = [0, 1, 0x1234_5678_9abc_def0, u64::MAX];
    let types = [0, 1, 2, (1 << 63) - 1, 1 << 63, u64::MAX];
    let masks = [0, 1, 63, 64, 1 << 63, u64::MAX];
    let mut result = String::new();
    for hwnd in windows {
        for kind in types {
            for method in masks {
                let trace = Rc::new(RefCell::new(State::default()));
                let host = Rc::new(RefCell::new(HostState {
                    mode: 0,
                    next: 0,
                    alive: BTreeSet::new(),
                    trace: trace.clone(),
                }));
                let mut held = Vec::new();
                {
                    let unit = GamepadCapture::new(
                        Script(trace.clone()),
                        Provider(host.clone()),
                        usize::try_from(hwnd).unwrap(),
                        kind,
                        method,
                    );
                    assert!(!unit.connected());
                    for phase in 0..2 {
                        let mut uuid = "x".repeat(if phase == 0 { 1 } else { 257 });
                        assert!(unit.request_uuid(&mut uuid).unwrap());
                        let features = unit.get_features();
                        let info = unit.get_info().unwrap();
                        writeln!(result, "{{\"hwnd\":{hwnd},\"gamepad_type\":{kind},\"method\":{method},\"phase\":{phase},\"uuid\":\"{uuid}\",\"features\":{features},\"info\":{info}}}").unwrap();
                        held.push(uuid);
                    }
                    assert!(!unit.connected());
                }
                assert_eq!(held.len(), 2);
                assert_eq!(held[0], held[1]);
                assert!(!held[0].is_empty());
                let state = trace.borrow();
                assert!(!state.module && state.alive.is_empty() && state.events.is_empty());
                let host = host.borrow();
                assert!(host.next == 0 && host.alive.is_empty());
            }
        }
    }
    result
}

#[test]
#[ignore = "requires complete original Gamepad class/API query collector; no desktop capture"]
fn matches_original_uuid_features_and_info_without_host_or_capture_owners() {
    use std::{env, fs, path::PathBuf, process::Command};
    let program = PathBuf::from(env::var_os("MFS_P1_GAMEPAD_METADATA_SOURCE").unwrap());
    let root = PathBuf::from(env::var_os("MFS_P1_GAMEPAD_METADATA_RECORDS").unwrap());
    fs::create_dir(&root).unwrap();
    let output = Command::new(program).output().unwrap();
    fs::write(root.join("source.jsonl"), &output.stdout).unwrap();
    fs::write(root.join("source.stderr"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "original metadata: {:?}",
        output.status
    );
    let source = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(source.lines().count(), 288);
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim(),
        "original_cases=144 phases=288 queries=864 input_owners=0 capture_units=0 native_capture=0"
    );
    let actual = report();
    fs::write(root.join("rust.jsonl"), &actual).unwrap();
    assert_eq!(source, actual);
    println!(
        "Gamepad metadata cases=144 phases=288 queries_per_side=864 exact_original_class_api=1 input_owners=0 capture_units=0 native_capture=0 full_P1_acceptance=0 passed"
    );
}
