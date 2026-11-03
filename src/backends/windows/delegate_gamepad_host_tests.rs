// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Opaque input-owner fixture bound to actual native delegate observations.
use super::*;
use crate::backends::windows::gamepad::{Host, InputOwner};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

struct State<'a> {
    ops: &'a Ops,
    directory: &'a Path,
    mode: u32,
    next: u32,
    live: BTreeSet<u32>,
    events: String,
}
impl State<'_> {
    fn record(&mut self, name: &str) {
        use std::fmt::Write;
        let (native, _) = self.ops.events();
        let count = |kind| native.iter().filter(|e| e.kind == kind).count();
        writeln!(
            self.events,
            "{name}\t{}\t{}\t{}\t{}\t{}",
            self.mode,
            count(1),
            count(5),
            count(3),
            u8::from(library::loaded(
                &self.directory.join("MaaWin32ControlUnit.dll")
            ))
        )
        .unwrap();
    }
}
type Shared<'a> = Rc<RefCell<State<'a>>>;
pub(super) struct Provider<'a>(Shared<'a>);
pub(super) struct Input<'a> {
    state: Shared<'a>,
    id: u32,
    connected: bool,
}
pub(super) struct Trace<'a> {
    state: Shared<'a>,
    path: PathBuf,
}
impl<'a> Trace<'a> {
    pub(super) fn new(ops: &'a Ops, directory: &'a Path, path: PathBuf) -> Self {
        Self {
            state: Rc::new(RefCell::new(State {
                ops,
                directory,
                mode: 0,
                next: 0,
                live: BTreeSet::new(),
                events: String::new(),
            })),
            path,
        }
    }
    pub(super) fn provider(&self) -> Provider<'a> {
        Provider(self.state.clone())
    }
    pub(super) fn mode(&self, value: u32) {
        assert!(value <= 3);
        self.state.borrow_mut().mode = value;
    }
    pub(super) fn save(&self) -> std::io::Result<()> {
        std::fs::write(&self.path, &self.state.borrow().events)
    }
    pub(super) fn closed(&self) -> bool {
        self.state.borrow().live.is_empty()
    }
}
impl Drop for Trace<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.save() {
            eprintln!("partial Gamepad host trace export failed: {error}");
        }
    }
}
impl<'a> Host for Provider<'a> {
    type Error = &'static str;
    type Input = Input<'a>;
    fn create(&mut self, gamepad_type: u64) -> Result<Self::Input, Self::Error> {
        let mut state = self.0.borrow_mut();
        state.record(&format!("input.create:{gamepad_type}"));
        if state.mode == 3 {
            return Err("host create");
        }
        state.next = state.next.checked_add(1).unwrap();
        let id = state.next;
        assert!(state.live.insert(id));
        state.record(&format!("input.new:{id}"));
        Ok(Input {
            state: self.0.clone(),
            id,
            connected: false,
        })
    }
}
impl InputOwner for Input<'_> {
    type Error = &'static str;
    fn connect(&mut self) -> Result<bool, Self::Error> {
        let mut state = self.state.borrow_mut();
        state.record(&format!("input.connect:{}", self.id));
        if state.mode == 2 {
            return Err("host connect");
        }
        self.connected = state.mode == 0;
        Ok(self.connected)
    }
    fn connected(&self) -> bool {
        self.state
            .borrow_mut()
            .record(&format!("input.connected:{}", self.id));
        self.connected
    }
    fn reset(&mut self) {
        self.state
            .borrow_mut()
            .record(&format!("input.reset:{}", self.id));
    }
}
impl Drop for Input<'_> {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        state.record(&format!("input.drop:{}", self.id));
        assert!(state.live.remove(&self.id));
    }
}
