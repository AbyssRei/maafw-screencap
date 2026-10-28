// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use std::{
    error::Error,
    ffi::{CStr, OsStr},
    os::windows::ffi::OsStrExt,
    ptr::null_mut,
};

#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
mod native {
    include!(concat!(env!("OUT_DIR"), "/windows_prototype.rs"));
}

pub(crate) struct Module(native::HMODULE);
impl Module {
    #[cfg(feature = "windows-print-window")]
    pub fn same_instance(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    pub fn load(path: &OsStr) -> Result<Self, Box<dyn Error>> {
        if !std::path::Path::new(path).is_absolute() {
            return Err("native parity DLL path must be absolute".into());
        }
        let wide: Vec<u16> = path.encode_wide().chain(Some(0)).collect();
        // SAFETY: Absolute NUL-terminated path; per-load safe dependency search.
        // No process-wide loader configuration is changed.
        let handle = unsafe { native::LoadLibraryExW(wide.as_ptr(), null_mut(), 0x100 | 0x1000) };
        if handle.is_null() {
            let error = unsafe { native::GetLastError() };
            return Err(format!("LoadLibraryExW failed: {error}").into());
        }
        Ok(Self(handle))
    }

    pub unsafe fn symbol<T: Copy>(&self, name: &CStr) -> Result<T, Box<dyn Error>> {
        if size_of::<T>() != size_of::<native::FARPROC>() {
            return Err("unexpected native function pointer representation".into());
        }
        // SAFETY: Live module owner, valid NUL-terminated name. Caller specifies
        // the exact header signature and retains this module through use.
        let symbol = unsafe { native::GetProcAddress(self.0, name.as_ptr().cast()) }
            .ok_or("native parity symbol missing")?;
        Ok(unsafe { std::mem::transmute_copy(&symbol) })
    }
}
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: Owns one loader reference. Native objects/function users must
        // retain this owner until their final call, callback and worker join.
        let _ = unsafe { native::FreeLibrary(self.0) };
    }
}
