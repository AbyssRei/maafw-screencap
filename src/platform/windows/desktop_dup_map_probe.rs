// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Scalar test observations over real native Desktop Duplication calls.
use super::native::*;
use std::{cell::Cell, marker::PhantomData, rc::Rc};
use windows_core::{HRESULT, Interface};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Trace {
    pub fault: u32,
    pub order: u32,
    pub acquires: u32,
    pub copies: u32,
    pub maps: u32,
    pub unmaps: u32,
    pub releases: u32,
    pub conversions: u32,
    pub acquired: i32,
    pub mapped_result: i32,
    pub released: i32,
    pub timeout: u32,
    pub subresource: u32,
    pub pitch: u32,
    pub thread: u32,
    pub context: usize,
    pub resource: usize,
    pub duplication: usize,
    pub mapped: bool,
    pub held: bool,
    pub violation: bool,
}
thread_local! {static ACTIVE: Cell<Option<Trace>> = const { Cell::new(None) };}
pub(crate) struct Guard(PhantomData<Rc<()>>);
impl Guard {
    pub fn begin(fault: u32) -> Self {
        assert!(fault <= 2);
        ACTIVE.with(|slot| {
            assert!(slot.get().is_none());
            slot.set(Some(Trace {
                fault,
                ..Trace::default()
            }));
        });
        Self(PhantomData)
    }
    pub fn trace(&self) -> Trace {
        ACTIVE.with(|slot| slot.get().unwrap())
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|slot| slot.set(None));
    }
}
fn update(operation: impl FnOnce(&mut Trace)) {
    ACTIVE.with(|slot| {
        if let Some(mut trace) = slot.get() {
            operation(&mut trace);
            slot.set(Some(trace));
        }
    });
}
fn event(code: u32, operation: impl FnOnce(&mut Trace)) {
    update(|trace| {
        // SAFETY: Read-only identity of the real current native caller.
        let thread = unsafe { GetCurrentThreadId() };
        if trace.thread == 0 {
            trace.thread = thread;
        } else {
            trace.violation |= trace.thread != thread;
        }
        if let Some(order) = trace
            .order
            .checked_mul(10)
            .and_then(|n| n.checked_add(code))
        {
            trace.order = order;
        } else {
            trace.violation = true;
        }
        operation(trace);
    });
}
pub(crate) fn acquiring(duplication: &IDXGIOutputDuplication) {
    event(1, |t| {
        t.acquires += 1;
        t.timeout = 2000;
        t.duplication = duplication.as_raw() as usize;
        t.violation |= t.held || t.mapped;
    });
}
pub(crate) fn acquired(code: HRESULT) {
    update(|t| {
        t.acquired = code.0;
        t.held = code.is_ok();
    });
}
pub(crate) fn copied(context: &ID3D11DeviceContext, texture: &ID3D11Texture2D) {
    event(2, |t| {
        t.copies += 1;
        t.context = context.as_raw() as usize;
        t.resource = texture.as_raw() as usize;
        t.violation |= !t.held || t.mapped;
    });
}
pub(crate) unsafe fn map(
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    output: &mut D3D11_MAPPED_SUBRESOURCE,
) -> HRESULT {
    let index = ACTIVE.with(|slot| {
        if slot.get().is_some_and(|t| t.fault == 1) {
            u32::MAX
        } else {
            0
        }
    });
    event(3, |t| {
        t.maps += 1;
        t.subresource = index;
        t.violation |= t.mapped
            || !t.held
            || t.context != context.as_raw() as usize
            || t.resource != texture.as_raw() as usize;
    });
    // SAFETY: Same real context/texture/output as production; the deliberate
    // invalid index asks native validation to reject before output consumption.
    let code = unsafe { context.Map(texture, index, D3D11_MAP_READ, 0, Some(output)) };
    if index != 0 && code.is_ok() {
        unsafe { context.Unmap(texture, index) };
        panic!("invalid-subresource desktop Map unexpectedly succeeded");
    }
    update(|t| {
        t.mapped_result = code.0;
        t.mapped = code.is_ok();
        t.pitch = if code.is_ok() { output.RowPitch } else { 0 };
    });
    code
}
pub(crate) fn unmapped(context: &ID3D11DeviceContext, texture: &ID3D11Texture2D) {
    // Drop observation: fixed scalar work, with no allocation, assertion or I/O.
    event(4, |t| {
        t.unmaps += 1;
        t.violation |= !t.mapped
            || !t.held
            || t.context != context.as_raw() as usize
            || t.resource != texture.as_raw() as usize;
        t.mapped = false;
    });
}
pub(crate) fn released(duplication: &IDXGIOutputDuplication, code: HRESULT) {
    event(5, |t| {
        t.releases += 1;
        t.released = code.0;
        t.violation |= !t.held || t.mapped || t.duplication != duplication.as_raw() as usize;
        t.held = false;
    });
}
pub(crate) fn conversion() -> bool {
    let mut fail = false;
    event(6, |t| {
        t.conversions += 1;
        t.violation |= t.mapped || t.held;
        fail = t.fault == 2;
    });
    fail
}
