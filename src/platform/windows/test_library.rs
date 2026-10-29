// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Explicit acceptance module ownership only, using generated Win32 bindings.
use std::{ffi::CStr, mem::size_of, os::windows::ffi::OsStrExt, path::Path};
#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
mod sys {
    include!(concat!(env!("OUT_DIR"), "/windows_prototype.rs"));
}
pub(crate) struct Module(sys::HMODULE);
impl Module {
    pub fn open(path: &Path) -> Self {
        assert!(path.is_absolute());
        let mut wide: Vec<_> = path.as_os_str().encode_wide().collect();
        assert!(!wide.contains(&0));
        wide.push(0);
        // SAFETY: Exact absolute, initialized terminated path from explicit
        // acceptance configuration. No process DLL search state changes.
        let module =
            unsafe { sys::LoadLibraryExW(wide.as_ptr(), std::ptr::null_mut(), 0x100 | 0x1000) };
        assert!(!module.is_null(), "load explicit native acceptance library");
        Self(module)
    }
    /// # Safety
    /// T must match the actual exported private C declaration. The module
    /// must outlive every pointer invocation and every native owner it releases.
    pub unsafe fn symbol<T: Copy>(&self, name: &CStr) -> T {
        assert_eq!(size_of::<T>(), size_of::<sys::FARPROC>());
        // SAFETY: Live module and terminated export name, borrowed for call.
        let pointer = unsafe { sys::GetProcAddress(self.0, name.as_ptr().cast()) }
            .expect("native acceptance export missing");
        // SAFETY: Caller establishes exact header/calling convention and
        // module lifetime; representation checked against generated FARPROC.
        unsafe { std::mem::transmute_copy(&pointer) }
    }
}
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: One owned load reference; all native owners/contexts ended first.
        assert_ne!(unsafe { sys::FreeLibrary(self.0) }, 0);
    }
}
