// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::windows::{
    gdi_api::{DcApi, SystemGdi},
    native::{BITMAPINFO, HBITMAP, HDC, HGDIOBJ, HWND, POINT, RECT},
    print_window_api::PrintApi,
};
use std::mem::MaybeUninit;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Event {
    pub operation: u32,
    pub popup: u32,
    pub result: i32,
    pub last_error: u32,
}
#[derive(Debug, Default)]
#[repr(C)]
pub struct Report {
    count: u32,
    overflow: u32,
    events: [Event; 16],
}
impl Report {
    pub fn events(&self) -> &[Event] {
        assert_eq!(self.overflow, 0, "query report overflow");
        &self.events[..usize::try_from(self.count).unwrap()]
    }
}
#[repr(C)]
struct Api {
    root: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    popup: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    valid: unsafe extern "C" fn(*mut c_void) -> i32,
    visible: unsafe extern "C" fn(*mut c_void) -> i32,
    origin: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
}
type End = unsafe extern "C" fn(*mut Report) -> i32;

impl Adapter {
    fn verify_print_query_layout(&self) -> Result<(), Box<dyn Error>> {
        // SAFETY: Exact private header signature, from this retained module.
        let layout: Layout = unsafe { self.module.symbol(c"mfs_print_query_layout")? };
        let mut actual = [0_u64; 12];
        // SAFETY: Twelve writable uint64 outputs with their stated capacity.
        if unsafe { layout(actual.as_mut_ptr(), actual.len()) } != 0 {
            return Err("PrintWindow query layout failed".into());
        }
        let expected = [
            size_of::<Event>(),
            align_of::<Event>(),
            offset_of!(Event, popup),
            offset_of!(Event, result),
            offset_of!(Event, last_error),
            size_of::<Report>(),
            align_of::<Report>(),
            offset_of!(Report, events),
            size_of::<Api>(),
            align_of::<Api>(),
            offset_of!(Api, origin),
            16,
        ]
        .map(|value| u64::try_from(value).unwrap());
        if actual != expected {
            return Err("PrintWindow query C/Rust layout differs".into());
        }
        Ok(())
    }
    pub fn print_query_reference(
        &self,
        hwnd: HWND,
        scenario: u32,
    ) -> Result<(Result<Image, i32>, Report), Box<dyn Error>> {
        self.verify_print_query_layout()?;
        type Reference = unsafe extern "C" fn(HWND, u32, *mut ImageSpan, *mut Report) -> i32;
        // SAFETY: Exact private header signature; initialized writable outputs.
        let reference: Reference = unsafe { self.module.symbol(c"mfs_print_query_reference")? };
        let mut image = ImageSpan::default();
        let mut report = Report::default();
        let status = unsafe { reference(hwnd, scenario, &mut image, &mut report) };
        Ok((self.adopt(status, image), report))
    }
    pub fn print_query_probe(
        &self,
        hwnd: HWND,
        scenario: u32,
    ) -> Result<Probe<'_>, Box<dyn Error>> {
        self.verify_print_query_layout()?;
        type Begin = unsafe extern "C" fn(HWND, u32) -> i32;
        type GetApi = unsafe extern "C" fn(*mut Api) -> i32;
        // SAFETY: Verified layout and exact private signatures. Resolve every
        // function before begin so a symbol failure cannot leave an active probe.
        let (begin, end, get_api): (Begin, End, GetApi) = unsafe {
            (
                self.module.symbol(c"mfs_print_query_begin")?,
                self.module.symbol(c"mfs_print_query_end")?,
                self.module.symbol(c"mfs_print_query_api")?,
            )
        };
        let mut api = MaybeUninit::<Api>::uninit();
        // SAFETY: Success initializes every non-null function pointer. Error
        // outputs are never interpreted as Rust function pointer values.
        if unsafe { get_api(api.as_mut_ptr()) } != 0 {
            return Err("PrintWindow query table failed".into());
        }
        let api = unsafe { api.assume_init() };
        // SAFETY: Probe retains the module and the caller-thread ownership.
        if unsafe { begin(hwnd, scenario) } != 0 {
            return Err("nested or invalid PrintWindow query probe".into());
        }
        Ok(Probe {
            _adapter: self,
            api,
            end,
            active: true,
        })
    }
}
pub struct Probe<'a> {
    _adapter: &'a Adapter,
    api: Api,
    end: End,
    active: bool,
}
impl Probe<'_> {
    pub fn finish(mut self) -> Result<Report, Box<dyn Error>> {
        self.active = false;
        let mut report = Report::default();
        // SAFETY: Consumes this same-thread begin exactly once, after capture.
        if unsafe { (self.end)(&mut report) } != 0 {
            return Err("PrintWindow query end failed".into());
        }
        Ok(report)
    }
}
impl Drop for Probe<'_> {
    fn drop(&mut self) {
        if self.active {
            let mut report = Report::default();
            // SAFETY: Unwind fallback owns the same-thread query state only;
            // no native resources or callbacks are retained by these queries.
            let _ = unsafe { (self.end)(&mut report) };
        }
    }
}

// SAFETY: Unmodified same-thread Win32 forwarding. Only PrintApi queries below
// are interposed; all DC/bitmap ownership and spans remain SystemGdi's contract.
unsafe impl DcApi for Probe<'_> {
    unsafe fn get_dc(&self, hwnd: HWND) -> HDC {
        unsafe { SystemGdi.get_dc(hwnd) }
    }
    unsafe fn create_dc(&self, dc: HDC) -> HDC {
        unsafe { SystemGdi.create_dc(dc) }
    }
    unsafe fn client_rect(&self, hwnd: HWND, rectangle: *mut RECT) -> i32 {
        unsafe { SystemGdi.client_rect(hwnd, rectangle) }
    }
    unsafe fn create_bitmap(&self, dc: HDC, width: i32, height: i32) -> HBITMAP {
        unsafe { SystemGdi.create_bitmap(dc, width, height) }
    }
    unsafe fn select(&self, dc: HDC, object: HGDIOBJ) -> HGDIOBJ {
        unsafe { SystemGdi.select(dc, object) }
    }
    unsafe fn delete_bitmap(&self, bitmap: HBITMAP) -> i32 {
        unsafe { SystemGdi.delete_bitmap(bitmap) }
    }
    unsafe fn delete_dc(&self, dc: HDC) -> i32 {
        unsafe { SystemGdi.delete_dc(dc) }
    }
    unsafe fn release_dc(&self, hwnd: HWND, dc: HDC) -> i32 {
        unsafe { SystemGdi.release_dc(hwnd, dc) }
    }
}
// SAFETY: Exact verified query table; queries retain no caller pointers. print
// and dibits forward the same checked native ownership/write spans unchanged.
unsafe impl PrintApi for Probe<'_> {
    unsafe fn is_window(&self, hwnd: HWND) -> i32 {
        unsafe { (self.api.valid)(hwnd) }
    }
    unsafe fn root_owner(&self, hwnd: HWND) -> HWND {
        unsafe { (self.api.root)(hwnd) }
    }
    unsafe fn last_popup(&self, hwnd: HWND) -> HWND {
        unsafe { (self.api.popup)(hwnd) }
    }
    unsafe fn visible(&self, hwnd: HWND) -> i32 {
        unsafe { (self.api.visible)(hwnd) }
    }
    unsafe fn origin(&self, hwnd: HWND, point: *mut POINT) -> i32 {
        unsafe { (self.api.origin)(hwnd, point.cast()) }
    }
    unsafe fn print(&self, hwnd: HWND, dc: HDC) -> i32 {
        unsafe { SystemGdi.print(hwnd, dc) }
    }
    unsafe fn dibits(
        &self,
        dc: HDC,
        bitmap: HBITMAP,
        height: u32,
        pixels: *mut c_void,
        info: *mut BITMAPINFO,
    ) -> i32 {
        unsafe { SystemGdi.dibits(dc, bitmap, height, pixels, info) }
    }
}
