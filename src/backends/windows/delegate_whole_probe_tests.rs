// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Nonowning diagnostics on a live original class, with an independent code
// lease ending before the original unit/loader. No class-layout field access.
use super::*;
use std::ffi::{CStr, c_char};
type Inspect = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
pub(super) struct Probe {
    _module: Module,
    inspect: Inspect,
}
impl Probe {
    pub(super) fn load(provider: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let module = Module::load(provider.as_os_str())?;
        // SAFETY: Original public version signature; retained module owns its
        // static terminated string until after this immediate comparison.
        let version: unsafe extern "C" fn() -> *const c_char =
            unsafe { module.symbol(c"MaaWin32ControlUnitGetVersion")? };
        let pointer = unsafe { version() };
        assert!(!pointer.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(pointer) }.to_bytes(),
            b"mfs-p1-whole-win32-class"
        );
        // SAFETY: Exact private whole-source.cpp signature, in retained code.
        let inspect = unsafe { module.symbol(c"mfs_whole_inspect")? };
        Ok(Self {
            _module: module,
            inspect,
        })
    }
    pub(super) fn inspect(&self, unit: NonNull<c_void>) -> [u64; 8] {
        let mut values = [0_u64; 8];
        // SAFETY: Caller keeps its unit alive and provides eight initialized
        // outputs. Inspection only checks RTTI/counters on this caller channel.
        assert_eq!(
            unsafe { (self.inspect)(unit.as_ptr(), values.as_mut_ptr(), values.len()) },
            0
        );
        assert_eq!(&values[..5], &[0x4d465357, 1, 0, 0, 1]);
        assert!(values[5] >= 8 && values[6] == 8);
        assert_eq!(
            values[7],
            u64::from(crate::platform::windows::manager::probe::thread())
        );
        values
    }
}
