// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Test-only observations of real D3D calls; never substitutes a COM object.
use super::native::*;
use std::{cell::Cell, marker::PhantomData, rc::Rc};
use windows_core::{HRESULT, Interface};
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Trace {
    pub fault: u32,
    pub order: u32,
    pub maps: u32,
    pub unmaps: u32,
    pub conversions: u32,
    pub cache_clones: u32,
    pub map_hresult: i32,
    pub subresource: u32,
    pub pitch: u32,
    pub context: usize,
    pub resource: usize,
    pub thread: u32,
    pub mapped: bool,
    pub violation: bool,
}
thread_local! {static ACTIVE: Cell<Option<Trace>> = const {Cell::new(None)};}
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
fn event(code: u32, update: impl FnOnce(&mut Trace)) {
    ACTIVE.with(|slot| {
        if let Some(mut trace) = slot.get() {
            // SAFETY: Read-only identity query on this exact native caller.
            let thread = unsafe { GetCurrentThreadId() };
            if trace.thread == 0 {
                trace.thread = thread;
            } else if trace.thread != thread {
                trace.violation = true;
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
            update(&mut trace);
            slot.set(Some(trace));
        }
    });
}
pub(super) fn copy(context: &ID3D11DeviceContext, texture: &ID3D11Texture2D) {
    event(1, |trace| {
        trace.context = context.as_raw() as usize;
        trace.resource = texture.as_raw() as usize;
    });
}
pub(super) unsafe fn map(
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    output: &mut D3D11_MAPPED_SUBRESOURCE,
) -> HRESULT {
    let fault = ACTIVE.with(|slot| slot.get().map_or(0, |trace| trace.fault));
    let index = if fault == 1 { u32::MAX } else { 0 };
    event(2, |trace| {
        trace.maps = trace.maps.saturating_add(1);
        trace.subresource = index;
        trace.violation |= trace.mapped
            || trace.context != context.as_raw() as usize
            || trace.resource != texture.as_raw() as usize;
    });
    // SAFETY: Same real owned texture/output as production. The explicit fault
    // changes only the subresource passed to OS validation, never its layout.
    let code = unsafe { context.Map(texture, index, D3D11_MAP_READ, 0, Some(output)) };
    if index != 0 && code.is_ok() {
        // A failed positive control must not leak a surprising successful Map
        // or let the original path consume an unsupported native output.
        unsafe { context.Unmap(texture, index) };
        panic!("native invalid-subresource Map unexpectedly succeeded");
    }
    ACTIVE.with(|slot| {
        if let Some(mut trace) = slot.get() {
            trace.map_hresult = code.0;
            trace.mapped = code.is_ok();
            trace.pitch = if code.is_ok() { output.RowPitch } else { 0 };
            slot.set(Some(trace));
        }
    });
    code
}
pub(super) fn unmapped(context: &ID3D11DeviceContext, texture: &ID3D11Texture2D) {
    // This path also runs during unwinding: fixed scalar observation only, no
    // allocation, formatting, assertion or fallible I/O replaces the primary error.
    event(5, |trace| {
        trace.unmaps = trace.unmaps.saturating_add(1);
        trace.violation |= !trace.mapped
            || trace.context != context.as_raw() as usize
            || trace.resource != texture.as_raw() as usize;
        trace.mapped = false;
    });
}
pub(crate) fn conversion() -> bool {
    let mut fail = false;
    event(3, |trace| {
        trace.conversions = trace.conversions.saturating_add(1);
        trace.violation |= !trace.mapped;
        fail = trace.fault == 2;
    });
    fail
}
pub(crate) fn cache_clone() {
    event(4, |trace| {
        trace.cache_clones = trace.cache_clones.saturating_add(1);
        trace.violation |= !trace.mapped;
    });
}
