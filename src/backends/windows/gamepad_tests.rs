// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::super::delegate_script::{Image, Script, Shared, State};
use super::*;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc, sync::Arc};
#[path = "../../../tests/upstream-parity/windows-delegate/gamepad-cases.rs"]
mod cases;

struct HostState {
    mode: u32,
    next: u8,
    alive: BTreeSet<u8>,
    trace: Shared,
}
type HostShared = Rc<RefCell<HostState>>;
struct Provider(HostShared);
struct Input {
    state: HostShared,
    id: u8,
    connected: bool,
}
impl Host for Provider {
    type Error = &'static str;
    type Input = Input;
    fn create(&mut self, gamepad_type: u64) -> Result<Input, Self::Error> {
        let mut s = self.0.borrow_mut();
        s.trace
            .borrow_mut()
            .events
            .push(format!("input.create:{gamepad_type}"));
        if s.mode == 3 {
            return Err("host create");
        }
        s.next = s.next.checked_add(1).unwrap();
        let id = s.next;
        assert!(s.alive.insert(id));
        s.trace.borrow_mut().events.push(format!("input.new:{id}"));
        Ok(Input {
            state: self.0.clone(),
            id,
            connected: false,
        })
    }
}
impl InputOwner for Input {
    type Error = &'static str;
    fn connect(&mut self) -> Result<bool, Self::Error> {
        let s = self.state.borrow();
        s.trace
            .borrow_mut()
            .events
            .push(format!("input.connect:{}", self.id));
        if s.mode == 2 {
            return Err("host connect");
        }
        self.connected = s.mode != 1;
        Ok(self.connected)
    }
    fn connected(&self) -> bool {
        let s = self.state.borrow();
        s.trace
            .borrow_mut()
            .events
            .push(format!("input.connected:{}", self.id));
        self.connected && s.mode != 4
    }
    fn reset(&mut self) {
        self.state
            .borrow()
            .trace
            .borrow_mut()
            .events
            .push(format!("input.reset:{}", self.id));
    }
}
impl Drop for Input {
    fn drop(&mut self) {
        let mut s = self.state.borrow_mut();
        s.trace
            .borrow_mut()
            .events
            .push(format!("input.drop:{}", self.id));
        assert!(s.alive.remove(&self.id));
    }
}
fn pixels(image: &Image) -> String {
    image.as_ref().map_or_else(
        || "empty".into(),
        |image| image.iter().map(|b| format!("{b:02x}")).collect(),
    )
}
fn capture_status(result: Result<bool, CaptureError<&str>>) -> u32 {
    match result {
        Ok(b) => u32::from(b),
        Err(CaptureError::Boundary(_)) => 2,
        Err(CaptureError::InvalidatedSourceView) => 4,
    }
}
fn report(case: cases::Case) -> String {
    use std::fmt::Write;
    let trace = Rc::new(RefCell::new(State::default()));
    let host = Rc::new(RefCell::new(HostState {
        mode: 0,
        next: 0,
        alive: BTreeSet::new(),
        trace: trace.clone(),
    }));
    let mut result = String::new();
    let mut held = Vec::new();
    let mut output = Some(Arc::try_new([99; 18]).unwrap());
    {
        let mut manager = GamepadCapture::new(
            Script(trace.clone()),
            Provider(host.clone()),
            usize::try_from(case.hwnd).unwrap(),
            case.gamepad_type,
            case.method,
        );
        writeln!(
            result,
            "initial_connected={}\ninitial_capture={}\ninitial_inactive={}",
            u8::from(manager.connected()),
            capture_status(manager.capture(&mut output)),
            capture_status(manager.inactive())
        )
        .unwrap();
        for phase in 0..2 {
            host.borrow_mut().mode = case.host[phase];
            {
                let mut s = trace.borrow_mut();
                s.flags = case.flags[phase];
                s.outcome = case.outcome;
                s.inactive = case.inactive;
            }
            let connected = match manager.connect() {
                Ok(b) => u32::from(b),
                Err(_) => 2,
            };
            writeln!(
                result,
                "connect{phase}={connected}\nconnected{phase}={}",
                u8::from(manager.connected())
            )
            .unwrap();
            let captured = capture_status(manager.capture(&mut output));
            let inactive = capture_status(manager.inactive());
            writeln!(
                result,
                "capture{phase}={captured}\noutput{phase}={}\ninactive{phase}={inactive}",
                pixels(&output)
            )
            .unwrap();
            held.push(output.clone());
        }
    }
    assert!(
        !trace.borrow().module && trace.borrow().alive.is_empty() && host.borrow().alive.is_empty()
    );
    for (index, image) in held.iter().enumerate() {
        writeln!(result, "held{index}={}", pixels(image)).unwrap();
    }
    writeln!(
        result,
        "events={}\nowners_closed=1",
        trace.borrow().events.join(",")
    )
    .unwrap();
    result
}

#[test]
#[ignore = "requires exact complete original Gamepad class/API and explicit host/DLL fixtures"]
fn matches_original_outer_host_failures_capture_retention_and_reset_before_teardown() {
    use std::{env, fs, path::PathBuf, process::Command};
    let program = PathBuf::from(env::var_os("MFS_P1_GAMEPAD_HOST_MODEL").unwrap());
    let root = PathBuf::from(env::var_os("MFS_P1_GAMEPAD_HOST_RECORDS").unwrap());
    fs::create_dir(&root).unwrap();
    let matrix = cases::matrix();
    for (index, case) in matrix.iter().copied().enumerate() {
        let output = Command::new(&program)
            .args(case.arguments())
            .output()
            .unwrap();
        fs::write(root.join(format!("{index}.stderr")), &output.stderr).unwrap();
        fs::write(root.join(format!("{index}.source")), &output.stdout).unwrap();
        assert!(
            output.status.success(),
            "original Gamepad {index}: {:?}",
            output.status
        );
        let source = String::from_utf8(output.stdout)
            .unwrap()
            .replace("\r\n", "\n");
        let actual = report(case);
        fs::write(root.join(format!("{index}.rust")), &actual).unwrap();
        assert_eq!(source, actual, "Gamepad outer case {index}: {case:?}");
    }
    println!(
        "Gamepad outer host cases={} exact_original_class_api=1 capture_delegate_reused=1 reset_before_capture_teardown=1 input_actions=0 native_capture=0 full_P1_acceptance=0 passed",
        matrix.len()
    );
}
