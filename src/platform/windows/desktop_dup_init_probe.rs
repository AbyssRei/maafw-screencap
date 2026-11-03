// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Caller-local controlled initialization failures; no screenshot or fallback.
use super::native::*;
use std::{cell::Cell, ffi::c_void, marker::PhantomData, rc::Rc};
use windows_core::{HRESULT, Interface};

pub(crate) type Attach = unsafe extern "C" fn(*mut c_void, u32, u32) -> i32;
pub(crate) type Read = unsafe extern "C" fn(u32, *mut u64, usize) -> i32;
pub(crate) const CODES: [HRESULT; 4] = [
    HRESULT(0x8000_4005_u32 as i32),
    HRESULT(0x8007_000e_u32 as i32),
    HRESULT(0x8000_4005_u32 as i32),
    HRESULT(0x8000_4002_u32 as i32),
];
#[derive(Clone, Copy, Default)]
pub(crate) struct Trace {
    pub mode: u32,
    pub order: u32,
    pub device_calls: u32,
    pub native_devices: u32,
    pub factory_calls: u32,
    pub native_factories: u32,
    pub device_result: i32,
    pub factory_result: i32,
    pub caller: u32,
    pub violation: bool,
    pub live: [[u64; 12]; 2],
}
#[derive(Clone, Copy)]
struct Active {
    trace: Trace,
    first_index: u32,
    attach: Attach,
    read: Read,
}
thread_local! { static ACTIVE: Cell<Option<Active>> = const { Cell::new(None) }; }
pub(crate) struct Guard(PhantomData<Rc<()>>);
impl Guard {
    pub fn begin(mode: u32, first_index: u32, attach: Attach, read: Read) -> Self {
        assert!(mode <= 4 && first_index <= 78);
        ACTIVE.with(|s| {
            assert!(s.get().is_none());
            s.set(Some(Active {
                trace: Trace {
                    mode,
                    ..Trace::default()
                },
                first_index,
                attach,
                read,
            }));
        });
        Self(PhantomData)
    }
    pub fn trace(&self) -> Trace {
        ACTIVE.with(|s| s.get().unwrap().trace)
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|s| s.set(None));
    }
}
fn update(f: impl FnOnce(&mut Active)) {
    ACTIVE.with(|s| {
        if let Some(mut active) = s.get() {
            f(&mut active);
            s.set(Some(active));
        }
    });
}
pub(crate) fn device(call: impl FnOnce() -> HRESULT) -> HRESULT {
    let mode = ACTIVE.with(|s| s.get().map(|a| a.trace.mode));
    update(|a| {
        // SAFETY: Read-only current caller identity; no host context mutation.
        a.trace.caller = unsafe { GetCurrentThreadId() };
        a.trace.violation |= a.trace.order != 0;
        a.trace.order = 1;
        a.trace.device_calls += 1;
        a.trace.native_devices += u32::from(a.trace.mode >= 2);
    });
    let code = match mode {
        Some(0 | 1) => CODES[usize::try_from(mode.unwrap()).unwrap()],
        _ => call(),
    };
    update(|a| a.trace.device_result = code.0);
    code
}
pub(crate) fn created(device: Option<&ID3D11Device>, context: Option<&ID3D11DeviceContext>) {
    update(|a| {
        if !(2..=3).contains(&a.trace.mode) {
            return;
        }
        let pointers = [
            device.map(Interface::as_raw),
            context.map(Interface::as_raw),
        ];
        for (kind, pointer) in pointers.into_iter().enumerate() {
            let Some(pointer) = pointer else {
                a.trace.violation = true;
                continue;
            };
            let index = a.first_index + u32::try_from(kind).unwrap();
            // SAFETY: Both COM owners are live on this caller. Native notifier
            // code/records remain loaded for process lifetime, including failure.
            a.trace.violation |=
                unsafe { (a.attach)(pointer, u32::try_from(kind).unwrap(), index) } != 0;
            a.trace.violation |=
                unsafe { (a.read)(index, a.trace.live[kind].as_mut_ptr(), 12) } != 0;
        }
    });
}
pub(crate) fn factory(
    call: impl FnOnce() -> windows_core::Result<IDXGIFactory>,
) -> windows_core::Result<IDXGIFactory> {
    let mode = ACTIVE.with(|s| s.get().map(|a| a.trace.mode));
    update(|a| {
        a.trace.violation |= a.trace.order != 1 || a.trace.mode < 2;
        a.trace.order = 12;
        a.trace.factory_calls += 1;
        a.trace.native_factories += u32::from(a.trace.mode == 4);
        a.trace.violation |= a.trace.caller != unsafe { GetCurrentThreadId() };
    });
    let result = match mode {
        Some(2 | 3) => Err(CODES[usize::try_from(mode.unwrap()).unwrap()].into()),
        _ => call(),
    };
    update(|a| a.trace.factory_result = result.as_ref().err().map_or(0, |e| e.code().0));
    result
}
