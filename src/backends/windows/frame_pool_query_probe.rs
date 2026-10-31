// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Test-only Win32 query interposition after an actual FramePool mapping.
use crate::platform::windows::wgc::native;
use std::{cell::Cell, ffi::c_void, marker::PhantomData, rc::Rc};
use windows_core::{BOOL, HRESULT};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Trace {
    pub case: u32,
    pub order: u32,
    pub win32: u32,
    pub dwm: i32,
}
thread_local! { static ACTIVE: Cell<Option<Trace>> = const { Cell::new(None) }; }
pub(super) struct Guard(PhantomData<Rc<()>>);
impl Guard {
    pub fn begin(case: u32) -> Self {
        assert!(case < 10);
        ACTIVE.with(|state| {
            assert!(state.get().is_none());
            state.set(Some(Trace {
                case,
                ..Trace::default()
            }));
        });
        Self(PhantomData)
    }
    pub fn trace(&self) -> Trace {
        ACTIVE.with(|state| state.get().unwrap())
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|state| state.set(None));
    }
}
fn enter(operation: u32) -> Option<u32> {
    ACTIVE.with(|state| {
        let mut trace = state.get()?;
        trace.order = trace
            .order
            .checked_mul(10)
            .unwrap()
            .checked_add(operation)
            .unwrap();
        state.set(Some(trace));
        Some(trace.case)
    })
}
fn error(win32: u32, dwm: Option<i32>) {
    ACTIVE.with(|state| {
        if let Some(mut trace) = state.get() {
            trace.win32 = win32;
            if let Some(code) = dwm {
                trace.dwm = code;
            }
            state.set(Some(trace));
        }
    });
}
fn invalid() -> native::HWND {
    // An intentionally invalid scalar handle goes only to OS validation;
    // Rust never dereferences it or claims a native owner for it.
    native::HWND(std::ptr::without_provenance_mut::<c_void>(1))
}

#[allow(non_snake_case)]
pub(super) unsafe fn GetClientRect(window: native::HWND, output: *mut native::RECT) -> BOOL {
    let case = enter(1);
    // SAFETY: Same live exact output as production. The failure case changes
    // only the opaque handle presented to OS validation.
    let result =
        unsafe { native::GetClientRect(if case == Some(1) { invalid() } else { window }, output) };
    let code = unsafe { native::GetLastError() };
    if result.as_bool() {
        if case == Some(5) {
            unsafe {
                (*output).right = (*output).left.checked_add(128).unwrap();
            }
        }
        if case == Some(6) {
            unsafe {
                (*output).bottom = (*output).top.checked_add(96).unwrap();
            }
        }
    } else {
        error(code, None);
    }
    // Preserve native last-error across bookkeeping and TLS access.
    unsafe { native::SetLastError(code) };
    result
}
#[allow(non_snake_case)]
pub(super) unsafe fn ClientToScreen(window: native::HWND, point: *mut native::POINT) -> BOOL {
    let case = enter(2);
    // SAFETY: Exact initialized input/output and OS-validated opaque handle.
    let result =
        unsafe { native::ClientToScreen(if case == Some(2) { invalid() } else { window }, point) };
    let code = unsafe { native::GetLastError() };
    if result.as_bool() {
        if case == Some(7) {
            unsafe {
                (*point).x = (*point).x.checked_add(128).unwrap();
            }
        }
        if case == Some(8) {
            unsafe {
                (*point).y = (*point).y.checked_add(96).unwrap();
            }
        }
        if case == Some(9) {
            unsafe {
                (*point).x = (*point).x.checked_sub(8).unwrap();
                (*point).y = (*point).y.checked_sub(8).unwrap();
            }
        }
    } else {
        error(code, None);
    }
    unsafe { native::SetLastError(code) };
    result
}
#[allow(non_snake_case)]
pub(super) unsafe fn DwmGetWindowAttribute(
    window: native::HWND,
    attribute: u32,
    output: *mut c_void,
    size: u32,
) -> HRESULT {
    let case = enter(3);
    // SAFETY: Unchanged attribute/output/span. Only explicitly injected cases
    // substitute an invalid opaque handle; the returned HRESULT remains real.
    let result = unsafe {
        native::DwmGetWindowAttribute(
            if matches!(case, Some(3 | 4)) {
                invalid()
            } else {
                window
            },
            attribute,
            output,
            size,
        )
    };
    let code = unsafe { native::GetLastError() };
    if result.is_err() {
        error(0, Some(result.0));
    }
    unsafe { native::SetLastError(code) };
    result
}
#[allow(non_snake_case)]
pub(super) unsafe fn GetWindowRect(window: native::HWND, output: *mut native::RECT) -> BOOL {
    let case = enter(4);
    // SAFETY: Exact writable rectangle and opaque handle validated by the OS.
    let result =
        unsafe { native::GetWindowRect(if case == Some(4) { invalid() } else { window }, output) };
    let code = unsafe { native::GetLastError() };
    if !result.as_bool() {
        error(code, None);
    }
    unsafe { native::SetLastError(code) };
    result
}
