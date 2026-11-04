// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Actual original class RTTI and zero invocation of excluded input/application
// boundaries. The extra diagnostic DLL lease ends before unit destruction.
use super::*;
use std::ffi::{CStr, c_char};
type Inspect = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
pub(super) struct Inspector {
    _module: Module,
    inspect: Inspect,
    rows: fs::File,
}
impl Inspector {
    pub(super) fn new(
        provider: &Path,
        directory: &Path,
        unit: NonNull<c_void>,
        method: u64,
    ) -> Result<Self> {
        let module = Module::load(provider.as_os_str())?;
        // SAFETY: Exact original public version signature; this retained module
        // owns its static terminated string through the immediate comparison.
        let version: unsafe extern "C" fn() -> *const c_char =
            unsafe { module.symbol(c"MaaWin32ControlUnitGetVersion")? };
        let pointer = unsafe { version() };
        assert!(!pointer.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(pointer) }.to_bytes(),
            b"mfs-p1-whole-win32-class"
        );
        // SAFETY: Private whole-source.cpp declaration, retained actual module.
        let inspect: Inspect = unsafe { module.symbol(c"mfs_whole_inspect")? };
        let mut result = Self {
            _module: module,
            inspect,
            rows: fs::File::create_new(directory.join("whole-source-types.tsv"))?,
        };
        result.record(0, unit)?;
        fs::write(
            directory.join("whole-source-selection.txt"),
            format!(
                "single_method_mask={method}\nselection_observer=0\noriginal_class_api=1\ninput_none=1\n"
            ),
        )?;
        Ok(result)
    }
    fn record(&mut self, stage: u32, unit: NonNull<c_void>) -> Result<()> {
        let mut values = [0_u64; 8];
        // SAFETY: Caller keeps the original unit alive through this synchronous
        // nonowning RTTI check and provides exactly eight initialized outputs.
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
        writeln!(
            self.rows,
            "{stage}\t{}",
            values.map(|v| v.to_string()).join("\t")
        )?;
        self.rows.flush()?;
        Ok(())
    }
    pub(super) fn finish(&mut self, unit: NonNull<c_void>) -> Result<()> {
        self.record(1, unit)
    }
}
