// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! P1 LibraryHolder's reachable fixed-name Windows module path. No unit ABI.
use std::{
    ffi::CStr,
    os::windows::ffi::OsStrExt,
    path::Path,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicU32, Ordering},
    },
};
#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
mod sys {
    include!(concat!(env!("OUT_DIR"), "/windows_delegate.rs"));
}

pub(crate) static CLOSE_ERROR: AtomicU32 = AtomicU32::new(0);
static CACHE: Mutex<Weak<Module>> = Mutex::new(Weak::new());
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    InvalidPath,
    Allocation,
    DifferentPath,
    Poisoned,
    Kernel(u32),
}

pub(crate) struct Location {
    names: [Vec<u16>; 4],
}
impl Location {
    pub(crate) fn new(directory: &Path) -> Result<Self, Error> {
        if !directory.is_absolute() {
            return Err(Error::InvalidPath);
        }
        let mut base = Vec::new();
        for unit in directory.as_os_str().encode_wide() {
            if unit == 0 {
                return Err(Error::InvalidPath);
            }
            base.try_reserve(1).map_err(|_| Error::Allocation)?;
            base.push(if unit == u16::from(b'/') {
                u16::from(b'\\')
            } else {
                unit
            });
        }
        if base.last() != Some(&u16::from(b'\\')) {
            base.try_reserve(1).map_err(|_| Error::Allocation)?;
            base.push(u16::from(b'\\'));
        }
        fn named(base: &[u16], suffix: &str) -> Result<Vec<u16>, Error> {
            let mut result = Vec::new();
            let size = base
                .len()
                .checked_add(suffix.len())
                .and_then(|n| n.checked_add(1))
                .ok_or(Error::Allocation)?;
            result
                .try_reserve_exact(size)
                .map_err(|_| Error::Allocation)?;
            result.extend_from_slice(base);
            result.extend(suffix.encode_utf16());
            result.push(0);
            Ok(result)
        }
        Ok(Self {
            names: [
                named(&base, "MaaWin32ControlUnit")?,
                named(&base, "MaaWin32ControlUnit.dll")?,
                named(&base, "libMaaWin32ControlUnit.dll")?,
                named(&base, "MaaWin32ControlUnit.")?,
            ],
        })
    }
}
struct Module {
    handle: sys::HMODULE,
    key: Vec<u16>,
}
// SAFETY: This owns only a process module load reference, not any unit, COM
// object or callback. Win32 load/symbol/free operate on process module refs.
// Every lease's final Arc release occurs under CACHE, matching source mutex
// ordering. Export invocation/lifetime is a separate unsafe caller obligation.
unsafe impl Send for Module {}
unsafe impl Sync for Module {}
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: One successful executable load; the final lease ends only
        // after its units. This is never called from a DllMain in this crate.
        if unsafe { sys::FreeLibrary(self.handle) } == 0 {
            let error = unsafe { sys::GetLastError() };
            let _ = CLOSE_ERROR.compare_exchange(0, error, Ordering::AcqRel, Ordering::Acquire);
        }
    }
}
pub(crate) struct Lease(Option<Arc<Module>>);
impl Lease {
    pub(crate) fn load(location: &Location) -> Result<Self, Error> {
        let mut cache = CACHE.lock().map_err(|_| Error::Poisoned)?;
        if let Some(module) = cache.upgrade() {
            if module.key != location.names[0] {
                return Err(Error::DifferentPath);
            }
            return Ok(Self(Some(module)));
        }
        let mut key = Vec::new();
        key.try_reserve_exact(location.names[0].len())
            .map_err(|_| Error::Allocation)?;
        key.extend_from_slice(&location.names[0]);
        let mut last = 0;
        // Original Boost Windows append_decorations tries all three names
        // with native flags=0, including the trailing dot to avoid automatic
        // .dll expansion. It does not modify process DLL search settings.
        for name in &location.names[1..] {
            // SAFETY: Fully initialized terminated absolute UTF-16 name;
            // executable DLL load with original flags and null reserved arg.
            let handle = unsafe { sys::LoadLibraryExW(name.as_ptr(), std::ptr::null_mut(), 0) };
            if !handle.is_null() {
                let module = Arc::try_new(Module { handle, key }).map_err(|_| Error::Allocation)?;
                *cache = Arc::downgrade(&module);
                return Ok(Self(Some(module)));
            }
            last = unsafe { sys::GetLastError() };
        }
        Err(Error::Kernel(last))
    }
    /// # Safety
    /// The address has no Rust ABI or type guarantee. An invocation must use
    /// its actual header and remain within this lease's lifetime. Unit native
    /// exceptions must be contained by an appropriate foreign-language bridge.
    pub(crate) unsafe fn symbol(&self, name: &CStr) -> Option<usize> {
        let module = self.0.as_ref().unwrap();
        // SAFETY: Module load reference remains live; terminated export name.
        unsafe { sys::GetProcAddress(module.handle, name.as_ptr().cast()) }
            .map(|function| function as *const () as usize)
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        // Standard Arc does the counting. Holding the same mutex through the
        // last Arc release prevents a new load racing ahead of final unload.
        let _guard = CACHE.lock().unwrap_or_else(|error| error.into_inner());
        drop(self.0.take());
    }
}
#[cfg(test)]
pub(crate) fn loaded(path: &Path) -> bool {
    let mut name: Vec<_> = path.as_os_str().encode_wide().collect();
    assert!(!name.contains(&0));
    name.push(0);
    // SAFETY: Borrowed observation only; this NEVER owns/releases the result.
    !unsafe { sys::GetModuleHandleW(name.as_ptr()) }.is_null()
}
