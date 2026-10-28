// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::{probe, *};
use crate::backends::windows::{
    gdi_api::{DcApi, SystemGdi},
    native::{BITMAPINFO, HBITMAP, HDC, HGDIOBJ, HWND, POINT, RECT},
    print_window_api::PrintApi,
};
use std::mem::MaybeUninit;

pub const READ_DESELECT: u32 = 14;
pub const READ_RESELECT: u32 = 15;
pub const POPUP_ONLY: u32 = 1 << 28;

#[repr(C)]
struct Api {
    print: unsafe extern "C" fn(HWND, HDC) -> i32,
    read: unsafe extern "C" fn(HDC, HBITMAP, u32, *mut c_void, *mut c_void) -> i32,
    deselect: unsafe extern "C" fn(HDC, HGDIOBJ) -> HGDIOBJ,
    reselect: unsafe extern "C" fn(HDC, HBITMAP) -> HGDIOBJ,
}

impl probe::Report {
    pub fn print_comparable(&self) -> Vec<probe::Event> {
        self.events()
            .iter()
            .copied()
            .filter(|event| ![READ_DESELECT, READ_RESELECT].contains(&event.operation))
            .map(|mut event| {
                // Success need not define GetLastError. Raw observations remain
                // in Report; failure error evidence is compared without changes.
                if event.result > 0 {
                    event.last_error = 0;
                }
                event
            })
            .collect()
    }
}

impl Adapter {
    fn verify_print_fault_layout(&self) -> Result<(), Box<dyn Error>> {
        self.verify_probe_layout()?;
        // SAFETY: Exact private C header signature from this retained module.
        let layout: Layout = unsafe { self.module.symbol(c"mfs_print_fault_layout")? };
        let mut actual = [0_u64; 9];
        // SAFETY: Nine writable uint64 outputs with the stated capacity.
        if unsafe { layout(actual.as_mut_ptr(), actual.len()) } != 0 {
            return Err("PrintWindow fault layout query failed".into());
        }
        let expected = [
            size_of::<Api>(),
            align_of::<Api>(),
            offset_of!(Api, print),
            offset_of!(Api, read),
            offset_of!(Api, deselect),
            offset_of!(Api, reselect),
            usize::try_from(READ_DESELECT)?,
            usize::try_from(READ_RESELECT)?,
            usize::try_from(POPUP_ONLY)?,
        ]
        .map(|value| u64::try_from(value).unwrap());
        if actual != expected {
            return Err("PrintWindow fault C/Rust layout or constants differ".into());
        }
        Ok(())
    }
    pub fn print_fault_reference(
        &self,
        hwnd: HWND,
        failures: u32,
    ) -> Result<(Result<Image, i32>, probe::Report), Box<dyn Error>> {
        self.verify_print_fault_layout()?;
        type Reference = unsafe extern "C" fn(HWND, u32, *mut ImageSpan, *mut probe::Report) -> i32;
        // SAFETY: Exact private header signature with initialized writable outputs.
        let reference: Reference = unsafe { self.module.symbol(c"mfs_print_fault_reference")? };
        let mut image = ImageSpan::default();
        let mut report = probe::Report::default();
        let status = unsafe { reference(hwnd, failures, &mut image, &mut report) };
        Ok((self.adopt(status, image), report))
    }
    pub fn print_fault_probe(&self, failures: u32) -> Result<Probe<'_>, Box<dyn Error>> {
        self.verify_print_fault_layout()?;
        type GetApi = unsafe extern "C" fn(*mut Api) -> i32;
        // SAFETY: Resolve all pointers before begin so a symbol failure cannot
        // leave native state active. Success writes all four non-null functions.
        let get_api: GetApi = unsafe { self.module.symbol(c"mfs_print_fault_api")? };
        let mut api = MaybeUninit::<Api>::uninit();
        if unsafe { get_api(api.as_mut_ptr()) } != 0 {
            return Err("PrintWindow fault API query failed".into());
        }
        let api = unsafe { api.assume_init() };
        Ok(Probe {
            shared: self.probe(failures)?,
            api,
        })
    }
}

pub struct Probe<'a> {
    shared: probe::Probe<'a>,
    api: Api,
}
impl Probe<'_> {
    pub fn finish(self) -> Result<probe::Report, Box<dyn Error>> {
        self.shared.finish()
    }
}

// SAFETY: Borrow one active caller-thread ledger and its module. Real Win32
// objects/spans follow DcApi; residual recovery happens only after recording
// capture's own cleanup attempts. The Adapter borrow prevents Send/Sync.
unsafe impl DcApi for Probe<'_> {
    unsafe fn get_dc(&self, hwnd: HWND) -> HDC {
        unsafe { self.shared.get_dc(hwnd) }
    }
    unsafe fn create_dc(&self, dc: HDC) -> HDC {
        unsafe { self.shared.create_dc(dc) }
    }
    unsafe fn client_rect(&self, hwnd: HWND, rect: *mut RECT) -> i32 {
        unsafe { self.shared.client_rect(hwnd, rect) }
    }
    unsafe fn create_bitmap(&self, dc: HDC, w: i32, h: i32) -> HBITMAP {
        unsafe { self.shared.create_bitmap(dc, w, h) }
    }
    unsafe fn select(&self, dc: HDC, object: HGDIOBJ) -> HGDIOBJ {
        unsafe { self.shared.select(dc, object) }
    }
    unsafe fn delete_bitmap(&self, bitmap: HBITMAP) -> i32 {
        unsafe { self.shared.delete_bitmap(bitmap) }
    }
    unsafe fn delete_dc(&self, dc: HDC) -> i32 {
        unsafe { self.shared.delete_dc(dc) }
    }
    unsafe fn release_dc(&self, hwnd: HWND, dc: HDC) -> i32 {
        unsafe { self.shared.release_dc(hwnd, dc) }
    }
}
// SAFETY: Same-thread queries are unchanged; the verified table reads only the
// checked 32-bit span/INFO. Read selections retain the final restore obligation.
unsafe impl PrintApi for Probe<'_> {
    unsafe fn is_window(&self, hwnd: HWND) -> i32 {
        unsafe { SystemGdi.is_window(hwnd) }
    }
    unsafe fn root_owner(&self, hwnd: HWND) -> HWND {
        unsafe { SystemGdi.root_owner(hwnd) }
    }
    unsafe fn last_popup(&self, hwnd: HWND) -> HWND {
        unsafe { SystemGdi.last_popup(hwnd) }
    }
    unsafe fn visible(&self, hwnd: HWND) -> i32 {
        unsafe { SystemGdi.visible(hwnd) }
    }
    unsafe fn origin(&self, hwnd: HWND, point: *mut POINT) -> i32 {
        unsafe { SystemGdi.origin(hwnd, point) }
    }
    unsafe fn print(&self, hwnd: HWND, dc: HDC) -> i32 {
        unsafe { (self.api.print)(hwnd, dc) }
    }
    unsafe fn unselect_for_read(&self, dc: HDC, previous: HGDIOBJ) -> HGDIOBJ {
        unsafe { (self.api.deselect)(dc, previous) }
    }
    unsafe fn reselect_after_read(&self, dc: HDC, bitmap: HBITMAP) -> HGDIOBJ {
        unsafe { (self.api.reselect)(dc, bitmap) }
    }
    unsafe fn dibits(
        &self,
        dc: HDC,
        bitmap: HBITMAP,
        height: u32,
        pixels: *mut c_void,
        info: *mut BITMAPINFO,
    ) -> i32 {
        unsafe { (self.api.read)(dc, bitmap, height, pixels, info.cast()) }
    }
}
