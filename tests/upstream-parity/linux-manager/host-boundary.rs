// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Source manager orchestration with explicit host input construction/init.
//! No device input actions, native host authorization or backend capture.
use super::*;

#[test]
fn retains_capture_and_feature_queries_after_host_input_failure_or_unknown_selector() {
    for method in [1, 2, 4, 3, u64::MAX] {
        for input_init in 0..3 {
            for input_throws in [false, true] {
                let mut script = Script {
                    input_init,
                    input_throws,
                    input_features: u64::MAX,
                    ..Script::default()
                };
                let mut manager = Manager::new(Config {
                    input_method: method,
                    ..config(PIPEWIRE)
                });
                let result = manager.connect(&mut script);
                let known = matches!(method, 1 | 2 | 4);
                if known && input_throws {
                    assert_eq!(result, Err("input allocation"));
                } else if known && input_init == 2 {
                    assert_eq!(result, Err("host input init exception"));
                } else {
                    assert_eq!(result, Ok(known && input_init == 0));
                }
                assert!(manager.capture.is_some());
                assert_eq!(manager.input.is_some(), known && !input_throws);
                assert_eq!(
                    manager.get_features(),
                    if known && !input_throws { 3 } else { 0 }
                );
                let mut expected = Vec::new();
                if method == 1 {
                    expected.extend(["client.create", "client.connect"]);
                }
                expected.push("capture.create");
                if known && !input_throws {
                    expected.extend(["input.create", "capture.init", "input.init"]);
                }
                assert_eq!(script.take(), expected);
                let mut output = 99;
                assert!(
                    manager
                        .capture_with(&mut output, |_, output| {
                            *output = 7;
                            Ok::<_, ()>(true)
                        })
                        .unwrap()
                );
                assert_eq!(output, 7);
                assert!(manager.inactive());
                let mut expected = vec!["capture.inactive"];
                if known && !input_throws {
                    expected.push("input.inactive");
                }
                assert_eq!(script.take(), expected);
            }
        }
    }
}

#[test]
fn host_wlr_input_retains_the_same_client_after_failed_connect_and_until_input_drop() {
    for method in [NONE, PIPEWIRE] {
        let mut script = Script {
            client_fails: true,
            ..Script::default()
        };
        let mut manager = Manager::new(Config {
            input_method: 1,
            ..config(method)
        });
        assert!(!manager.connect(&mut script).unwrap());
        assert_eq!(script.take(), ["client.create", "client.connect"]);
        assert_eq!(Rc::strong_count(manager.client.as_ref().unwrap()), 1);
        assert!(manager.capture.is_none() && manager.input.is_none());
        // Source retains the failed client and skips its connect on this call.
        // The explicit host script chooses true; no real Wlr input is implied.
        assert!(manager.connect(&mut script).unwrap());
        assert_eq!(
            script.take(),
            [
                "capture.create",
                "input.create",
                "capture.init",
                "input.init"
            ]
        );
        assert_eq!(Rc::strong_count(manager.client.as_ref().unwrap()), 2);
        drop(manager);
        assert_eq!(script.take(), ["capture.drop", "input.drop", "client.drop"]);
    }
}

#[test]
#[ignore = "explicit original manager bodies, real C++ shared owners/OpenCV, scripted host boundaries"]
fn matches_original_host_input_order_failures_features_and_retained_capture() {
    use std::{fmt::Write, path::PathBuf, process::Command};
    let program = std::env::var_os("MFS_P1_LINUX_MANAGER_MODEL").unwrap();
    let directory =
        PathBuf::from(std::env::var_os("MFS_P1_LINUX_HOST_INPUT_MODEL_ARTIFACTS").unwrap());
    std::fs::create_dir(&directory).unwrap();
    let mut paths = vec![
        b"socket".to_vec(),
        b"a\"\\\n b".to_vec(),
        Vec::new(),
        b"embedded\0suffix".to_vec(),
    ];
    if cfg!(target_os = "linux") {
        paths.push(vec![b'/', 0xff, 0x80]);
    }
    let hex = |bytes: &[u8]| {
        let mut result = String::new();
        for byte in bytes {
            write!(&mut result, "{byte:02x}").unwrap();
        }
        result
    };
    let mut cases = 0;
    for input_method in [NONE, 1, 2, 4, 3, u64::MAX] {
        for method in [NONE, WLR, EXT_IMAGE, PIPEWIRE] {
            for client_fails in [false, true] {
                for init_fails in [false, true] {
                    for input_throws in [false, true] {
                        for input_init in 0..3 {
                            for outcome in [0, 1, 3, 4] {
                                let socket = paths[cases % paths.len()].clone();
                                let input_path = paths[(cases + 1) % paths.len()].clone();
                                let input_features = [0, 1, 2, 3, 4, u64::MAX, 1 << 63][cases % 7];
                                let arguments = [
                                    method.to_string(),
                                    "-1".into(),
                                    "7".into(),
                                    u8::from(!client_fails).to_string(),
                                    u8::from(!init_fails).to_string(),
                                    u8::from(input_throws).to_string(),
                                    outcome.to_string(),
                                    hex(&socket),
                                    input_method.to_string(),
                                    input_init.to_string(),
                                    input_features.to_string(),
                                    hex(&input_path),
                                ];
                                let native =
                                    Command::new(&program).args(&arguments).output().unwrap();
                                std::fs::write(
                                    directory.join(format!("case-{cases}.arguments.txt")),
                                    arguments.join("\n"),
                                )
                                .unwrap();
                                std::fs::write(
                                    directory.join(format!("case-{cases}.original.txt")),
                                    &native.stdout,
                                )
                                .unwrap();
                                std::fs::write(
                                    directory.join(format!("case-{cases}.stderr.txt")),
                                    &native.stderr,
                                )
                                .unwrap();
                                assert!(
                                    native.status.success(),
                                    "original host-boundary case {cases}: {:?}",
                                    native.status
                                );
                                assert!(native.stderr.is_empty());
                                let mut script = Script {
                                    client_fails,
                                    init_fails,
                                    input_throws,
                                    input_init,
                                    input_features,
                                    ..Script::default()
                                };
                                let mut manager = Manager::new(Config {
                                    method,
                                    input_method,
                                    wlr_socket: socket,
                                    pipewire_fd: -1,
                                    pipewire_node: 7,
                                    uinput_path: std::borrow::Cow::Owned(input_path.clone()),
                                    eis_socket: input_path,
                                    uinput_width: 17,
                                    uinput_height: 19,
                                    ..Config::default()
                                });
                                let mut observed = format!(
                                    "initial={}\nfeatures_initial={}\n",
                                    u8::from(manager.connected()),
                                    manager.get_features()
                                );
                                let mut uuid = Vec::new();
                                let success = manager.request_uuid(&mut uuid).unwrap();
                                writeln!(
                                    &mut observed,
                                    "uuid_success={}\nuuid={}",
                                    u8::from(success),
                                    hex(&uuid)
                                )
                                .unwrap();
                                let mut output = 99;
                                for attempt in 0..2 {
                                    let connected =
                                        manager.connect(&mut script).map_or(2, u8::from);
                                    writeln!(&mut observed, "connect{attempt}={connected}\nconnected{attempt}={}\nfeatures{attempt}={}", u8::from(manager.connected()), manager.get_features()).unwrap();
                                    let captured = manager
                                        .capture_with(&mut output, |unit, output| {
                                            script.events.borrow_mut().push("capture.call");
                                            if unit.method == NONE
                                                || (unit.method == WLR && client_fails)
                                            {
                                                return Ok(false);
                                            }
                                            match outcome {
                                                0 => Ok(false),
                                                1 => {
                                                    *output = 7;
                                                    Ok(true)
                                                }
                                                3 => Err("scripted backend exception"),
                                                4 => {
                                                    *output = 7;
                                                    Ok(false)
                                                }
                                                _ => unreachable!(),
                                            }
                                        })
                                        .map_or(2, u8::from);
                                    writeln!(&mut observed, "capture{attempt}={captured}\noutput{attempt}={output}\ninactive{attempt}={}", u8::from(manager.inactive())).unwrap();
                                }
                                drop(manager);
                                writeln!(&mut observed, "events={}", script.take().join(","))
                                    .unwrap();
                                std::fs::write(
                                    directory.join(format!("case-{cases}.rust.txt")),
                                    &observed,
                                )
                                .unwrap();
                                assert_eq!(
                                    String::from_utf8(native.stdout)
                                        .unwrap()
                                        .replace("\r\n", "\n"),
                                    observed,
                                    "host-boundary case {cases}"
                                );
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 2304);
    eprintln!(
        "Linux manager host boundary cases=2304 source_body_comparisons=2304 two_connects=4608 retained_capture_and_features=1 input_actions=0 native_capture=0 passed"
    );
}
