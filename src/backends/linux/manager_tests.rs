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
        })
    }
    fn create_none_input(&mut self) -> Result<Owner, Self::Error> {
        if self.input_throws {
            return Err("input allocation");
        }
        Ok(self.owner("input.create", "input.drop"))
    }
    fn init_capture(&mut self, _: &mut Unit) -> Result<bool, Self::Error> {
        self.events.borrow_mut().push("capture.init");
        Ok(!self.init_fails)
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
    }
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
fn uuid_preserves_linux_path_quotes_bytes_and_signed_fd_identity() {
    for method in [WLR, EXT_IMAGE] {
        let mut cfg = config(method);
        cfg.wlr_socket = vec![b'a', b'"', b'\\', 0xff, b'\n'];
        let manager = Manager::<Script>::new(cfg);
        let mut output = vec![99];
        assert!(manager.request_uuid(&mut output).unwrap());
        assert_eq!(
            output,
            [
                b'"', b'a', b'\\', b'"', b'\\', b'\\', 0xff, b'\n', b'"', b'_', b'-', b'1'
            ]
        );
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
