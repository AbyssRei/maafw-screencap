// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::Adapter;
use std::{
    error::Error,
    mem::{offset_of, size_of},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Step {
    pub outcome: u32,
    pub reserved: u32,
    pub elapsed_ns: u64,
    pub image_drop_ns: u64,
}
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct Script {
    pub flags: u64,
    pub valid: u32,
    pub dpi_success: u32,
    pub fail_create: u32,
    pub reconnect: u32,
    pub null_window: u32,
    pub reserved: u32,
    pub steps: [[Step; 4]; 6],
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Event {
    pub kind: u32,
    pub method: u32,
    pub call: u32,
    pub reserved: u32,
    pub time_ns: u64,
}
#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Report {
    pub connect_status: u32,
    pub selected: u32,
    pub connected_before: u32,
    pub connected_after: u32,
    pub capture_status: [u32; 2],
    pub output_tokens: [u32; 2],
    pub reconnect_status: u32,
    pub selected_after_reconnect: u32,
    pub trace_count: u32,
    pub overflow: u32,
    pub events: [Event; 256],
}
impl Default for Report {
    fn default() -> Self {
        Self {
            connect_status: 0,
            selected: 0,
            connected_before: 0,
            connected_after: 0,
            capture_status: [0; 2],
            output_tokens: [0; 2],
            reconnect_status: 0,
            selected_after_reconnect: 0,
            trace_count: 0,
            overflow: 0,
            events: [Event::default(); 256],
        }
    }
}
#[derive(Debug, Default)]
#[repr(C)]
pub(crate) struct GdiReport {
    pub connected: u32,
    pub captured: u32,
    pub selected: u32,
    pub calls_after_connect: u32,
    pub calls: u32,
    pub threads: [u32; 3],
    pub dpi_v2: [u32; 3],
    pub reserved: u32,
}
impl Adapter {
    fn verify_manager_layout(&self) -> Result<(), Box<dyn Error>> {
        type Layout = unsafe extern "C" fn(*mut u64, usize) -> i32;
        // SAFETY: Exact private manager-script.h signatures in this module.
        let layout: Layout = unsafe { self.module.symbol(c"mfs_manager_script_layout")? };
        let mut actual = [0; 10];
        // SAFETY: Fixed initialized layout array with its full capacity.
        if unsafe { layout(actual.as_mut_ptr(), actual.len()) } != 0 {
            return Err("manager script native layout query failed".into());
        }
        let expected = [
            size_of::<Script>(),
            offset_of!(Script, steps),
            size_of::<Step>(),
            offset_of!(Step, elapsed_ns),
            size_of::<Report>(),
            offset_of!(Report, events),
            size_of::<Event>(),
            offset_of!(Event, time_ns),
            size_of::<GdiReport>(),
            offset_of!(GdiReport, dpi_v2),
        ]
        .map(|n| u64::try_from(n).unwrap());
        if actual != expected {
            return Err("manager script native layout mismatch".into());
        }
        Ok(())
    }
    pub fn manager_script(&self, input: &Script) -> Result<Report, Box<dyn Error>> {
        self.verify_manager_layout()?;
        type Run = unsafe extern "C" fn(*const Script, *mut Report) -> i32;
        // SAFETY: Exact verified private C boundary in this retained module.
        let run: Run = unsafe { self.module.symbol(c"mfs_manager_script")? };
        let mut report = Report::default();
        // SAFETY: Verified C layout, immutable input and exclusive initialized
        // output span. The call owns no input pointer after synchronous return.
        let status = unsafe { run(input, &mut report) };
        if status != 0 {
            return Err(format!("manager source script bridge: {status}").into());
        }
        if report.trace_count > 256 || report.overflow != 0 {
            return Err("manager source trace overflow".into());
        }
        Ok(report)
    }
    pub fn manager_gdi_reference(
        &self,
        window: *mut std::ffi::c_void,
    ) -> Result<(super::Image, GdiReport), Box<dyn Error>> {
        self.verify_manager_layout()?;
        type Run = unsafe extern "C" fn(
            *mut std::ffi::c_void,
            *mut super::ImageSpan,
            *mut GdiReport,
        ) -> i32;
        // SAFETY: Exact private C header, verified image/report layouts.
        let run: Run = unsafe { self.module.symbol(c"mfs_manager_gdi_reference")? };
        let mut image = super::ImageSpan::default();
        let mut report = GdiReport::default();
        // SAFETY: Nonowning live fixture HWND, exclusive initialized outputs;
        // synchronous source capture transfers one owned CPU image on success.
        let status = unsafe { run(window, &mut image, &mut report) };
        let image = self
            .adopt(status, image)
            .map_err(|code| format!("manager source GDI: {code}"))?;
        Ok((image, report))
    }
}
