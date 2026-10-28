// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
#[cfg(any(feature = "windows-gdi", feature = "windows-screen-dc"))]
use crate::backends::windows::gdi_api::GdiApi;
use crate::backends::windows::{
    gdi_api::DcApi,
    native::{HBITMAP, HDC, HGDIOBJ, HWND, RECT},
};
use std::mem::MaybeUninit;

pub const GET_DC: u32 = 1;
pub const CREATE_DC: u32 = 2;
pub const CLIENT_RECT: u32 = 3;
pub const CREATE_BITMAP: u32 = 4;
pub const SELECT: u32 = 5;
pub const COPY: u32 = 6;
pub const OBJECT: u32 = 7;
pub const READ: u32 = 8;
pub const CONVERT: u32 = 9;
pub const RESTORE: u32 = 10;
pub const DELETE_BITMAP: u32 = 11;
pub const DELETE_DC: u32 = 12;
pub const RELEASE_DC: u32 = 13;
pub const BITMAP_24: u32 = 1 << 29;
pub const SHORT_READ: u32 = 1 << 30;
pub fn fault(operation: u32) -> u32 {
    1 << (operation - 1)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Event {
    pub operation: u32,
    pub result: i32,
    pub last_error: u32,
}

#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct Report {
    pub count: u32,
    pub overflow: u32,
    pub before: u32,
    pub after_capture: u32,
    pub after_fixture: u32,
    pub fixture_errors: u32,
    pub bitmap_depth: u32,
    pub events: [Event; 32],
}

impl Report {
    pub fn events(&self) -> &[Event] {
        &self.events[..usize::try_from(self.count).unwrap()]
    }
    pub fn check_fixture(&self) {
        assert_eq!(self.overflow, 0);
        assert_eq!(self.fixture_errors, 0, "{self:?}");
        assert_eq!(
            self.after_fixture, self.before,
            "fixture must reclaim separately recorded residual resources: {self:?}"
        );
    }
    pub fn comparable(&self) -> Vec<Event> {
        // GetObjectW is the Rust layout validation absent in the original.
        self.events()
            .iter()
            .copied()
            .filter(|event| event.operation != OBJECT)
            .collect()
    }
}

#[repr(C)]
struct Api {
    get_dc: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    create_dc: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    client_rect: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
    create_bitmap: unsafe extern "C" fn(*mut c_void, i32, i32) -> *mut c_void,
    select: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
    copy: unsafe extern "C" fn(*mut c_void, i32, i32, *mut c_void) -> i32,
    object: unsafe extern "C" fn(*mut c_void, i32, *mut c_void) -> i32,
    read: unsafe extern "C" fn(*mut c_void, i32, *mut c_void) -> i32,
    delete_bitmap: unsafe extern "C" fn(*mut c_void) -> i32,
    delete_dc: unsafe extern "C" fn(*mut c_void) -> i32,
    release_dc: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
}
type Begin = unsafe extern "C" fn(u32) -> i32;
type End = unsafe extern "C" fn(*mut Report) -> i32;
type GetApi = unsafe extern "C" fn(*mut Api) -> i32;
type ProbeReference = unsafe extern "C" fn(*mut c_void, u32, *mut ImageSpan, *mut Report) -> i32;
#[cfg(feature = "windows-screen-dc")]
type ScreenCopy = unsafe extern "C" fn(*mut c_void, i32, i32, *mut c_void, i32, i32) -> i32;

impl Adapter {
    pub(super) fn verify_probe_layout(&self) -> Result<(), Box<dyn Error>> {
        // SAFETY: Exact signatures from gdi-probe.h; the module is retained.
        let layout: Layout = unsafe { self.module.symbol(c"mfs_gdi_probe_layout")? };
        let mut actual = [0_u64; 26];
        // SAFETY: All 26 output elements are writable and have the stated width.
        if unsafe { layout(actual.as_mut_ptr(), actual.len()) } != 0 {
            return Err("probe layout query failed".into());
        }
        let mut expected = [0_u64; 26];
        for (slot, value) in expected.iter_mut().zip([
            size_of::<Event>(),
            align_of::<Event>(),
            offset_of!(Event, result),
            offset_of!(Event, last_error),
            size_of::<Report>(),
            align_of::<Report>(),
            offset_of!(Report, events),
            size_of::<Api>(),
            align_of::<Api>(),
            offset_of!(Api, release_dc),
        ]) {
            *slot = u64::try_from(value)?;
        }
        expected[10..].copy_from_slice(
            &[
                GET_DC,
                CREATE_DC,
                CLIENT_RECT,
                CREATE_BITMAP,
                SELECT,
                COPY,
                OBJECT,
                READ,
                CONVERT,
                RESTORE,
                DELETE_BITMAP,
                DELETE_DC,
                RELEASE_DC,
                BITMAP_24,
                SHORT_READ,
                32,
            ]
            .map(u64::from),
        );
        if actual != expected {
            return Err("probe C/Rust layout or operation values differ".into());
        }
        Ok(())
    }

    pub fn probe(&self, failures: u32) -> Result<Probe<'_>, Box<dyn Error>> {
        self.verify_probe_layout()?;
        // SAFETY: These signatures match the private C header.
        let (begin, end, get_api): (Begin, End, GetApi) = unsafe {
            (
                self.module.symbol(c"mfs_gdi_probe_begin")?,
                self.module.symbol(c"mfs_gdi_probe_end")?,
                self.module.symbol(c"mfs_gdi_probe_api")?,
            )
        };
        let mut api = MaybeUninit::<Api>::uninit();
        // SAFETY: The C function writes all non-null function pointers on success.
        if unsafe { get_api(api.as_mut_ptr()) } != 0 {
            return Err("probe API query failed".into());
        }
        // SAFETY: Exact native layout was verified and get_api initialized every
        // function pointer. No partial/error output is interpreted as Rust values.
        let api = unsafe { api.assume_init() };
        #[cfg(feature = "windows-screen-dc")]
        // SAFETY: Exact additional screen-parity.h signature, resolved before
        // begin so a missing symbol cannot leave a native active probe behind.
        let screen_copy = unsafe { self.module.symbol(c"mfs_screen_probe_copy")? };
        // SAFETY: Begin is caller-thread-affine; Probe retains that module and
        // cannot be sent/shared because its Adapter contains raw native handles.
        if unsafe { begin(failures) } != 0 {
            return Err("nested or invalid probe".into());
        }
        Ok(Probe {
            adapter: self,
            api,
            end,
            active: true,
            #[cfg(feature = "windows-screen-dc")]
            screen_copy,
        })
    }

    pub fn reference_probe(
        &self,
        hwnd: *mut c_void,
        failures: u32,
    ) -> Result<(Result<Image, i32>, Report), Box<dyn Error>> {
        self.verify_probe_layout()?;
        // SAFETY: Signature and outputs match gdi-probe.h, and the module is live.
        let reference: ProbeReference = unsafe { self.module.symbol(c"mfs_gdi_probe_reference")? };
        let mut image = ImageSpan::default();
        let mut report = Report::default();
        // SAFETY: Opaque HWND is validated by the source's native calls. Both
        // output descriptors are fully writable and used only after the call.
        let status = unsafe { reference(hwnd, failures, &mut image, &mut report) };
        Ok((self.adopt(status, image), report))
    }
    #[cfg(feature = "windows-screen-dc")]
    pub fn screen_reference_probe(
        &self,
        hwnd: *mut c_void,
        failures: u32,
    ) -> Result<(Result<Image, i32>, Report), Box<dyn Error>> {
        self.verify_probe_layout()?;
        // SAFETY: Exact screen-parity.h signature; both writable descriptors
        // match verified native layouts and the module remains live.
        let reference: ProbeReference =
            unsafe { self.module.symbol(c"mfs_screen_probe_reference")? };
        let mut image = ImageSpan::default();
        let mut report = Report::default();
        let status = unsafe { reference(hwnd, failures, &mut image, &mut report) };
        Ok((self.adopt(status, image), report))
    }
}

pub struct Probe<'a> {
    adapter: &'a Adapter,
    api: Api,
    end: End,
    active: bool,
    #[cfg(feature = "windows-screen-dc")]
    screen_copy: ScreenCopy,
}

impl Probe<'_> {
    pub fn finish(mut self) -> Result<Report, Box<dyn Error>> {
        self.active = false;
        let mut report = Report::default();
        // SAFETY: This owns one successful begin on the current thread. Capture
        // resources borrowed this Probe and have finished their own attempts.
        if unsafe { (self.end)(&mut report) } != 0 {
            return Err("probe finalization failed".into());
        }
        Ok(report)
    }
}

impl Drop for Probe<'_> {
    fn drop(&mut self) {
        if self.active {
            let mut report = Report::default();
            // SAFETY: Unwind fallback consumes the same-thread fixture once.
            let _ = unsafe { (self.end)(&mut report) };
        }
        let _ = &self.adapter;
    }
}

// SAFETY: This borrows an active same-thread native probe. Each table function
// has the exact C header signature, returns real GDI objects on success and
// confines writes to the same spans as Win32. Fixture residuals are reclaimed
// only after recording the implementation's cleanup, never during capture.
unsafe impl DcApi for Probe<'_> {
    unsafe fn get_dc(&self, hwnd: HWND) -> HDC {
        unsafe { (self.api.get_dc)(hwnd) }
    }
    unsafe fn create_dc(&self, dc: HDC) -> HDC {
        unsafe { (self.api.create_dc)(dc) }
    }
    unsafe fn client_rect(&self, hwnd: HWND, rectangle: *mut RECT) -> i32 {
        unsafe { (self.api.client_rect)(hwnd, rectangle.cast()) }
    }
    unsafe fn create_bitmap(&self, dc: HDC, width: i32, height: i32) -> HBITMAP {
        unsafe { (self.api.create_bitmap)(dc, width, height) }
    }
    unsafe fn select(&self, dc: HDC, object: HGDIOBJ) -> HGDIOBJ {
        unsafe { (self.api.select)(dc, object) }
    }
    unsafe fn delete_bitmap(&self, bitmap: HBITMAP) -> i32 {
        unsafe { (self.api.delete_bitmap)(bitmap) }
    }
    unsafe fn delete_dc(&self, dc: HDC) -> i32 {
        unsafe { (self.api.delete_dc)(dc) }
    }
    unsafe fn release_dc(&self, hwnd: HWND, dc: HDC) -> i32 {
        unsafe { (self.api.release_dc)(hwnd, dc) }
    }
}

// SAFETY: Same-thread native probe table preserves the GDI read/write spans.
#[cfg(any(feature = "windows-gdi", feature = "windows-screen-dc"))]
unsafe impl GdiApi for Probe<'_> {
    unsafe fn copy(&self, destination: HDC, width: i32, height: i32, source: HDC) -> i32 {
        unsafe { (self.api.copy)(destination, width, height, source) }
    }
    unsafe fn object(&self, bitmap: HBITMAP, length: i32, output: *mut c_void) -> i32 {
        unsafe { (self.api.object)(bitmap, length, output) }
    }
    unsafe fn read(&self, bitmap: HBITMAP, length: i32, output: *mut c_void) -> i32 {
        unsafe { (self.api.read)(bitmap, length, output) }
    }
}

#[cfg(feature = "windows-screen-dc")]
// SAFETY: Same-thread native table retains real DC/bitmap ownership. Copy uses
// the screen origin directly and borrows no native objects beyond the call.
unsafe impl crate::backends::windows::gdi_api::ScreenDcApi for Probe<'_> {
    unsafe fn copy_screen(
        &self,
        destination: HDC,
        width: i32,
        height: i32,
        source: HDC,
        x: i32,
        y: i32,
    ) -> i32 {
        // SAFETY: Caller retains both DCs and selected destination bitmap.
        unsafe { (self.screen_copy)(destination, width, height, source, x, y) }
    }
}
