// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Shared explicit module/unit fixture; no native capture or input.
use crate::backends::windows::delegate::Boundary;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc, sync::Arc};

pub(super) type Image = Option<Arc<[u8; 18]>>;
#[derive(Default)]
pub(super) struct State {
    pub(super) flags: u32,
    pub(super) outcome: u32,
    pub(super) inactive: u32,
    pub(super) next: u8,
    pub(super) module: bool,
    pub(super) alive: BTreeSet<u8>,
    pub(super) events: Vec<String>,
    pub(super) fail_loader: bool,
    pub(super) fail_view: bool,
}
pub(super) type Shared = Rc<RefCell<State>>;
pub(super) struct Loader {
    state: Shared,
    unit: Option<u8>,
}
impl Drop for Loader {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        if let Some(unit) = self.unit {
            state.events.push(format!("unit.drop:{unit}"));
            assert!(state.alive.remove(&unit));
        }
        if state.module {
            state.events.push("module.unload".into());
            state.module = false;
        }
    }
}
pub(super) struct Script(pub(super) Shared);
impl Boundary for Script {
    type Loader = Loader;
    type Handle = u8;
    type View = u8;
    type Image = Image;
    type Error = &'static str;
    fn new_loader(&mut self) -> Result<Loader, Self::Error> {
        if self.0.borrow().fail_loader {
            return Err("loader allocation");
        }
        Ok(Loader {
            state: self.0.clone(),
            unit: None,
        })
    }
    fn load(&mut self, _: &mut Loader) -> Result<bool, Self::Error> {
        let mut state = self.0.borrow_mut();
        assert!(!state.module);
        state.events.push("module.load".into());
        if state.flags & 32 != 0 {
            return Err("load");
        }
        if state.flags & 1 != 0 {
            return Ok(false);
        }
        state.module = true;
        state.events.push("symbol.create".into());
        state.events.push("symbol.destroy".into());
        if state.flags & 6 != 0 {
            state.module = false;
            state.events.push("module.unload".into());
            return Ok(false);
        }
        Ok(true)
    }
    fn create(
        &mut self,
        loader: &mut Loader,
        hwnd: usize,
        method: u64,
        mouse: u64,
        keyboard: u64,
    ) -> Result<Option<u8>, Self::Error> {
        let mut state = self.0.borrow_mut();
        state
            .events
            .push(format!("unit.create:{hwnd}:{method}:{mouse}:{keyboard}"));
        assert_eq!((mouse, keyboard), (0, 0));
        if state.flags & 64 != 0 {
            return Err("create");
        }
        if state.flags & 8 != 0 {
            return Ok(None);
        }
        state.next = state.next.checked_add(1).unwrap();
        let unit = state.next;
        assert!(state.alive.insert(unit));
        loader.unit = Some(unit);
        Ok(Some(unit))
    }
    fn view(&mut self, loader: &Loader, handle: u8) -> Result<u8, Self::Error> {
        assert_eq!(loader.unit, Some(handle));
        if self.0.borrow().fail_view {
            return Err("view allocation");
        }
        Ok(handle)
    }
    fn connect(&mut self, loader: &mut Loader, view: &u8) -> Result<bool, Self::Error> {
        assert_eq!(loader.unit.as_ref(), Some(view));
        let mut state = self.0.borrow_mut();
        state.events.push(format!("unit.connect:{view}"));
        if state.flags & 128 != 0 {
            return Err("connect");
        }
        Ok(state.flags & 16 == 0)
    }
    fn capture(
        &mut self,
        loader: &mut Loader,
        view: &u8,
        output: &mut Image,
    ) -> Result<bool, Self::Error> {
        assert_eq!(loader.unit.as_ref(), Some(view));
        let mut state = self.0.borrow_mut();
        assert!(state.module && state.alive.contains(view));
        state.events.push(format!("unit.capture:{view}"));
        match state.outcome {
            0 => return Ok(false),
            2 => {
                *output = None;
                return Ok(true);
            }
            3 => return Err("capture"),
            _ => {}
        }
        let mut pixels = [0; 18];
        for pixel in pixels.as_chunks_mut::<3>().0 {
            pixel.copy_from_slice(&[7, *view, 11]);
        }
        *output = Some(Arc::try_new(pixels).unwrap());
        if state.outcome == 5 {
            return Err("capture after output");
        }
        Ok(state.outcome != 4)
    }
    fn inactive(&mut self, loader: &mut Loader, view: &u8) -> Result<bool, Self::Error> {
        assert_eq!(loader.unit.as_ref(), Some(view));
        let mut state = self.0.borrow_mut();
        assert!(state.module && state.alive.contains(view));
        state.events.push(format!("unit.inactive:{view}"));
        if state.inactive == 2 {
            return Err("inactive");
        }
        Ok(state.inactive == 1)
    }
}
