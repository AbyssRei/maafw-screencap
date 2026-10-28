// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::native::*;
use std::ffi::c_void;

pub(crate) fn set_per_monitor_v2() -> Result<isize, u32> {
    // SAFETY: Source's caller-thread DPI change, no borrowed memory. U leaves
    // the successful context active. A later P3 change must restore explicitly.
    let previous =
        unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    if previous.is_null() {
        // SAFETY: Thread-local native observation, no pointer inputs.
        Err(unsafe { GetLastError() })
    } else {
        Ok(previous as isize)
    }
}
pub(crate) fn valid(window: *mut c_void) -> bool {
    // SAFETY: Nonowning HWND query, validated by the OS without dereference.
    unsafe { IsWindow(window) != 0 }
}

#[cfg(test)]
pub(crate) mod probe {
    use super::*;
    pub fn current() -> isize {
        // SAFETY: Native opaque caller-context query, no ownership transfer.
        unsafe { GetThreadDpiAwarenessContext() as isize }
    }
    pub fn set(context: isize) -> bool {
        // SAFETY: An opaque native context constant or a prior context returned
        // by this thread; no Rust memory is dereferenced through the handle.
        !unsafe { SetThreadDpiAwarenessContext(context as *mut c_void) }.is_null()
    }
    pub fn unaware() -> isize {
        DPI_AWARENESS_CONTEXT_UNAWARE as isize
    }
    pub fn is_v2() -> bool {
        // SAFETY: Compare native opaque DPI handles on this caller.
        unsafe {
            AreDpiAwarenessContextsEqual(
                GetThreadDpiAwarenessContext(),
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            ) != 0
        }
    }
    pub fn thread() -> u32 {
        // SAFETY: Native caller identity query with no pointers.
        unsafe { GetCurrentThreadId() }
    }
}
