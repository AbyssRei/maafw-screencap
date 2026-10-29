// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use std::{cell::RefCell, rc::Rc};

type Events = Rc<RefCell<Vec<&'static str>>>;
struct Owner {
    events: Events,
    drop_event: &'static str,
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.events.borrow_mut().push(self.drop_event);
    }
}
struct Unit {
    _owner: Owner,
    _client: Option<Rc<Owner>>,
    method: u64,
}
#[derive(Default)]
struct Script {
    events: Events,
    client_fails: bool,
    init_fails: bool,
    input_throws: bool,
}
impl Script {
    fn owner(&self, event: &'static str, drop_event: &'static str) -> Owner {
        self.events.borrow_mut().push(event);
        Owner {
            events: self.events.clone(),
            drop_event,
        }
    }
    fn take(&self) -> Vec<&'static str> {
        std::mem::take(&mut *self.events.borrow_mut())
    }
}
impl Factory for Script {
    type Client = Rc<Owner>;
    type Unit = Unit;
    type Input = Owner;
    type Error = &'static str;
    fn create_client(&mut self) -> Result<Rc<Owner>, Self::Error> {
        Ok(Rc::new(self.owner("client.create", "client.drop")))
    }
    fn connect_client(&mut self, _: &mut Rc<Owner>, _: &[u8]) -> Result<bool, Self::Error> {
        self.events.borrow_mut().push("client.connect");
        Ok(!self.client_fails)
    }
    fn create_capture(
        &mut self,
        config: &Config,
        client: Option<&Rc<Owner>>,
    ) -> Result<Unit, Self::Error> {
        assert_eq!(client.is_some(), config.method == WLR);
        Ok(Unit {
            _owner: self.owner("capture.create", "capture.drop"),
            _client: client.cloned(),
            method: config.method,
        })
    }
    fn create_none_input(&mut self) -> Result<Owner, Self::Error> {
        if self.input_throws {
            return Err("input allocation");
        }
        Ok(self.owner("input.create", "input.drop"))
    }
    fn init_capture(&mut self, unit: &mut Unit) -> Result<bool, Self::Error> {
        self.events.borrow_mut().push("capture.init");
        Ok(unit.method != PIPEWIRE || !self.init_fails)
    }
    fn init_none_input(&mut self, _: &mut Owner) -> Result<bool, Self::Error> {
        self.events.borrow_mut().push("input.init");
        Ok(true) // Real NoneInput inherits the original true init.
    }
    fn inactive_capture(&mut self, _: &mut Unit) {
        self.events.borrow_mut().push("capture.inactive");
    }
    fn inactive_none_input(&mut self, _: &mut Owner) {
        self.events.borrow_mut().push("input.inactive");
    }
}
fn config(method: u64) -> Config {
    Config {
        method,
        wlr_socket: b"socket".to_vec(),
        pipewire_fd: -1,
        pipewire_node: 7,
        ..Config::default()
    }
}

#[test]
fn parsed_creation_and_queries_do_not_connect_or_touch_the_callers_output() {
    let mut script = Script::default();
    let json =
        br#"{"screencap_method":4,"input_method":0,"pw_node_id":9,"wlr_socket_path":"a\u0000b"}"#;
    let mut manager = Manager::<Script>::from_config_json(Some(json)).unwrap();
    assert!(!manager.connected());
    assert_eq!(manager.get_features(), 0);
    let mut output = vec![7, 9, 11];
    let pointer = output.as_ptr();
    assert!(
        !manager
            .capture_with(&mut output, |_, _| -> Result<bool, ()> {
                panic!("unconnected backend call")
            })
            .unwrap()
    );
    assert_eq!(output.as_ptr(), pointer);
    assert_eq!(output, [7, 9, 11]);
    let mut uuid = Vec::new();
    assert!(manager.request_uuid(&mut uuid).unwrap());
    assert_eq!(uuid, b"-1_9_-1");
    assert_eq!(
        Config::parse_c_string(Some(&manager.get_info().unwrap()))
            .unwrap()
            .wlr_socket,
        b"a\0b"
    );
    assert!(manager.inactive(&mut script));
    drop(manager);
    assert!(script.take().is_empty());
    assert!(Manager::<Script>::from_config_json(None).is_err());
    assert!(Manager::<Script>::from_config_json(Some(b"{}")).is_err());
}

#[test]
fn retained_input_metadata_does_not_enable_out_of_scope_input_control() {
    let mut script = Script::default();
    let mut manager = Manager::<Script>::from_config_json(Some(
        br#"{"screencap_method":1,"input_method":2,"uinput_path":"custom"}"#,
    ))
    .unwrap();
    let mut uuid = Vec::new();
    manager.request_uuid(&mut uuid).unwrap();
    assert_eq!(uuid, b"_custom");
    assert!(!manager.connect(&mut script).unwrap());
    assert!(!manager.connected());
    assert!(script.take().is_empty());
}

#[test]
fn failed_client_is_retained_and_not_reconnected_by_the_next_manager_connect() {
    let mut script = Script {
        client_fails: true,
        ..Script::default()
    };
    let mut manager = Manager::new(config(WLR));
    assert!(!manager.connect(&mut script).unwrap());
    assert!(!manager.connected());
    assert_eq!(script.take(), ["client.create", "client.connect"]);
    assert!(manager.connect(&mut script).unwrap());
    assert!(manager.connected());
    assert_eq!(Rc::strong_count(manager.client.as_ref().unwrap()), 2);
    assert_eq!(
        script.take(),
        [
            "capture.create",
            "input.create",
            "capture.init",
            "input.init"
        ]
    );
    let mut output = 99;
    assert!(
        !manager
            .capture_with(&mut output, |_, _| Ok::<_, ()>(false))
            .unwrap()
    );
    assert_eq!(output, 99);
    drop(manager);
    assert_eq!(script.take(), ["capture.drop", "client.drop", "input.drop"]);
}
#[test]
fn reconnect_releases_capture_then_input_and_reuses_the_existing_client() {
    let mut script = Script::default();
    let mut manager = Manager::new(config(WLR));
    assert!(manager.connect(&mut script).unwrap());
    script.take();
    assert!(manager.connect(&mut script).unwrap());
    assert_eq!(
        script.take(),
        [
            "capture.drop",
            "input.drop",
            "capture.create",
            "input.create",
            "capture.init",
            "input.init"
        ]
    );
}
#[test]
fn failed_init_retains_forwarding_and_inactive_orders_both_selected_owners() {
    let mut script = Script {
        init_fails: true,
        ..Script::default()
    };
    let mut manager = Manager::new(config(PIPEWIRE));
    assert!(!manager.connect(&mut script).unwrap());
    assert!(!manager.connected());
    assert_eq!(
        script.take(),
        ["capture.create", "input.create", "capture.init"]
    );
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
    assert!(manager.inactive(&mut script));
    assert_eq!(script.take(), ["capture.inactive", "input.inactive"]);
}
#[test]
fn input_construction_error_keeps_the_capture_and_does_not_rewrite_backend_failure_output() {
    let mut script = Script {
        input_throws: true,
        ..Script::default()
    };
    let mut manager = Manager::new(config(NONE));
    assert_eq!(manager.connect(&mut script), Err("input allocation"));
    assert!(!manager.connected());
    assert_eq!(script.take(), ["capture.create"]);
    let mut output = 99;
    assert!(
        !manager
            .capture_with(&mut output, |_, output| {
                *output = 7;
                Ok::<_, ()>(false)
            })
            .unwrap()
    );
    assert_eq!(output, 7);
    assert!(manager.inactive(&mut script));
    assert_eq!(script.take(), ["capture.inactive"]);
}
#[test]
fn pipewire_prerequisite_is_and_and_ext_image_remains_unimplemented_in_u() {
    for (method, fd, node, expected) in [
        (PIPEWIRE, -1, 0, false),
        (PIPEWIRE, -1, 7, true),
        (PIPEWIRE, 0, 0, true),
        (PIPEWIRE, 5, 7, true),
        (NONE, -1, 0, true),
        (EXT_IMAGE, -1, 0, false),
        (u64::MAX, 5, 7, false),
    ] {
        let mut script = Script::default();
        let mut cfg = config(method);
        cfg.pipewire_fd = fd;
        cfg.pipewire_node = node;
        let mut manager = Manager::new(cfg);
        assert_eq!(manager.connect(&mut script).unwrap(), expected);
        assert_eq!(manager.connected(), expected);
        assert_eq!(manager.capture.is_some(), expected);
        assert_eq!(manager.input.is_some(), expected);
    }
}
#[test]
fn uuid_preserves_socket_string_bytes_and_signed_fd_identity() {
    for method in [WLR, EXT_IMAGE] {
        let mut cfg = config(method);
        cfg.wlr_socket = vec![b'a', b'"', b'\\', 0xff, b'\n'];
        let manager = Manager::<Script>::new(cfg);
        let mut output = vec![99];
        assert!(manager.request_uuid(&mut output).unwrap());
        assert_eq!(output, [b'a', b'"', b'\\', 0xff, b'\n', b'_', b'-', b'1']);
    }
    let mut cfg = config(PIPEWIRE);
    cfg.pipewire_fd = i32::MIN;
    cfg.pipewire_node = u32::MAX;
    let manager = Manager::<Script>::new(cfg);
    let mut output = vec![99];
    assert!(manager.request_uuid(&mut output).unwrap());
    assert_eq!(output, b"-2147483648_4294967295_-1");
    for method in [NONE, u64::MAX] {
        let manager = Manager::<Script>::new(config(method));
        assert!(manager.request_uuid(&mut output).unwrap());
        assert_eq!(output, b"-1_-1");
    }
}

#[test]
#[ignore = "requires explicitly built original Linux manager model and real OpenCV"]
fn matches_fixed_source_manager_reconnect_forwarding_and_uuid() {
    use std::{fmt::Write, path::PathBuf, process::Command};
    let program = std::env::var_os("MFS_P1_LINUX_MANAGER_MODEL").unwrap();
    let directory =
        PathBuf::from(std::env::var_os("MFS_P1_LINUX_MANAGER_MODEL_ARTIFACTS").unwrap());
    std::fs::create_dir(&directory).unwrap();
    let mut paths = vec![b"socket".to_vec(), b"a\"\\\n b".to_vec(), Vec::new()];
    if cfg!(target_os = "linux") {
        paths.push(vec![b'/', 0xff, 0x80]);
    }
    let mut cases = 0;
    for method in [NONE, WLR, EXT_IMAGE, PIPEWIRE, 3, u64::MAX] {
        for (fd, node) in [(-1, 0), (-1, 7), (0, 0), (i32::MIN, u32::MAX)] {
            for client_fails in [false, true] {
                for init_fails in [false, true] {
                    for input_throws in [false, true] {
                        for outcome in 0..5 {
                            let socket = paths[cases % paths.len()].clone();
                            let hex = |bytes: &[u8]| {
                                let mut result = String::new();
                                for byte in bytes {
                                    write!(&mut result, "{byte:02x}").unwrap();
                                }
                                result
                            };
                            let arguments = [
                                method.to_string(),
                                fd.to_string(),
                                node.to_string(),
                                u8::from(!client_fails).to_string(),
                                u8::from(!init_fails).to_string(),
                                u8::from(input_throws).to_string(),
                                outcome.to_string(),
                                hex(&socket),
                            ];
                            let native = Command::new(&program).args(&arguments).output().unwrap();
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
                                "source model failed case {cases}: {:?}",
                                native.status
                            );
                            assert!(native.stderr.is_empty());
                            let mut script = Script {
                                client_fails,
                                init_fails,
                                input_throws,
                                ..Script::default()
                            };
                            let mut manager = Manager::new(Config {
                                method,
                                wlr_socket: socket,
                                pipewire_fd: fd,
                                pipewire_node: node,
                                ..Config::default()
                            });
                            let mut observed =
                                format!("initial={}\n", u8::from(manager.connected()));
                            let mut uuid = b"unchanged".to_vec();
                            let uuid_success = manager.request_uuid(&mut uuid).unwrap();
                            writeln!(
                                &mut observed,
                                "uuid_success={}\nuuid={}",
                                u8::from(uuid_success),
                                hex(&uuid)
                            )
                            .unwrap();
                            let mut output = 99;
                            for attempt in 0..2 {
                                let connected = manager.connect(&mut script).map_or(2, u8::from);
                                writeln!(
                                    &mut observed,
                                    "connect{attempt}={connected}\nconnected{attempt}={}",
                                    u8::from(manager.connected())
                                )
                                .unwrap();
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
                                            2 => {
                                                *output = 0;
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
                                writeln!(
                                    &mut observed,
                                    "capture{attempt}={captured}\noutput{attempt}={output}"
                                )
                                .unwrap();
                                writeln!(
                                    &mut observed,
                                    "inactive{attempt}={}",
                                    u8::from(manager.inactive(&mut script))
                                )
                                .unwrap();
                            }
                            drop(manager);
                            writeln!(&mut observed, "events={}", script.take().join(",")).unwrap();
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
                                "source manager case {cases}"
                            );
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 960);
    eprintln!(
        "Linux manager source model cases=960 source_body_comparisons=960 two_connects=1920 real_mat_outputs=1 scripted_backends=1 passed"
    );
}
