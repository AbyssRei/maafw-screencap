// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use crate::test_support::native_module::Module;
use std::{
    error::Error,
    mem::{align_of, offset_of, size_of},
};
type Layout = unsafe extern "C" fn(*mut u64, usize) -> i32;
pub struct Adapter {
    module: Module,
}
impl Adapter {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path =
            std::env::var_os("MFS_GDI_PARITY_LIBRARY").ok_or("MFS_GDI_PARITY_LIBRARY required")?;
        Ok(Self {
            module: Module::load(&path)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Event {
    pub operation: u32,
    pub error: u32,
    pub argument: i64,
    pub result: i64,
}

#[derive(Debug)]
#[repr(C)]
pub struct Report {
    count: u32,
    overflow: u32,
    events: [Event; 64],
    pub snapshots: [[u64; 5]; 4],
}
impl Default for Report {
    fn default() -> Self {
        Self {
            count: 0,
            overflow: 0,
            events: [Event::default(); 64],
            snapshots: [[0; 5]; 4],
        }
    }
}
impl Report {
    pub fn events(&self) -> &[Event] {
        assert_eq!(self.overflow, 0);
        &self.events[..usize::try_from(self.count).unwrap()]
    }
}

impl Adapter {
    pub fn pseudo_script(&self, scenario: u32) -> Result<Report, Box<dyn Error>> {
        type Script = unsafe extern "C" fn(u32, *mut Report) -> i32;
        // SAFETY: Exact signatures from pseudo-script.h. The script is complete
        // on return; it retains no module callbacks or native resources.
        let (layout, script): (Layout, Script) = unsafe {
            (
                self.module.symbol(c"mfs_pseudo_script_layout")?,
                self.module.symbol(c"mfs_pseudo_script")?,
            )
        };
        let mut actual = [0; 11];
        if unsafe { layout(actual.as_mut_ptr(), actual.len()) } != 0 {
            return Err("pseudo script layout query failed".into());
        }
        let expected = [
            size_of::<Event>(),
            align_of::<Event>(),
            offset_of!(Event, error),
            offset_of!(Event, argument),
            offset_of!(Event, result),
            size_of::<Report>(),
            align_of::<Report>(),
            offset_of!(Report, overflow),
            offset_of!(Report, events),
            offset_of!(Report, snapshots),
            64,
        ]
        .map(|value| u64::try_from(value).unwrap());
        if actual != expected {
            return Err("pseudo script C/Rust layout differs".into());
        }
        let mut report = Report::default();
        // SAFETY: Fully initialized writable output matches the verified layout.
        if unsafe { script(scenario, &mut report) } != 0 {
            return Err("original pseudo script failed".into());
        }
        Ok(report)
    }
}
