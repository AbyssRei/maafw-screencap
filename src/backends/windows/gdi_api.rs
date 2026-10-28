// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::native::*;
#[cfg(feature = "windows-gdi")]
use std::ffi::c_void;

/// The native boundary used by the same GDI capture/resource implementation.
/// SystemGdi calls Win32 directly; tests can observe/inject responses while
/// acquiring real native objects. No fault policy enters product capture.
///
/// # Safety
/// Successful handles must have the corresponding Win32 ownership, and all
/// calls stay on their owning thread. Raw outputs must write only their stated
/// span and must not retain pointers. Failures must preserve GetLastError until
/// the caller observes it. A borrowed API must outlive all of its resources.
pub(super) unsafe trait DcApi {
    unsafe fn get_dc(&self, hwnd: HWND) -> HDC;
    unsafe fn create_dc(&self, dc: HDC) -> HDC;
    unsafe fn client_rect(&self, hwnd: HWND, rectangle: *mut RECT) -> i32;
    unsafe fn create_bitmap(&self, dc: HDC, width: i32, height: i32) -> HBITMAP;
    unsafe fn select(&self, dc: HDC, object: HGDIOBJ) -> HGDIOBJ;
    unsafe fn delete_bitmap(&self, bitmap: HBITMAP) -> i32;
    unsafe fn delete_dc(&self, dc: HDC) -> i32;
    unsafe fn release_dc(&self, hwnd: HWND, dc: HDC) -> i32;
}

#[cfg(feature = "windows-gdi")]
/// # Safety
/// DcApi ownership applies; copy/object/read use exactly the stated writable spans.
pub(super) unsafe trait GdiApi: DcApi {
    unsafe fn copy(&self, destination: HDC, width: i32, height: i32, source: HDC) -> i32;
    unsafe fn object(&self, bitmap: HBITMAP, length: i32, output: *mut c_void) -> i32;
    unsafe fn read(&self, bitmap: HBITMAP, length: i32, output: *mut c_void) -> i32;
}

pub(super) struct SystemGdi;

// SAFETY: These are direct Win32 calls with the same signatures/ownership.
// Pointer and handle validity is required at each call site in the capture.
unsafe impl DcApi for SystemGdi {
    unsafe fn get_dc(&self, hwnd: HWND) -> HDC {
        // SAFETY: Caller supplies an opaque HWND; Win32 validates it.
        unsafe { GetDC(hwnd) }
    }
    unsafe fn create_dc(&self, dc: HDC) -> HDC {
        // SAFETY: Caller retains the source DC throughout this call.
        unsafe { CreateCompatibleDC(dc) }
    }
    unsafe fn client_rect(&self, hwnd: HWND, rectangle: *mut RECT) -> i32 {
        // SAFETY: Caller supplies a writable RECT and an OS HWND.
        unsafe { GetClientRect(hwnd, rectangle) }
    }
    unsafe fn create_bitmap(&self, dc: HDC, width: i32, height: i32) -> HBITMAP {
        // SAFETY: Live DC and positive dimensions are checked by the caller.
        unsafe { CreateCompatibleBitmap(dc, width, height) }
    }
    unsafe fn select(&self, dc: HDC, object: HGDIOBJ) -> HGDIOBJ {
        // SAFETY: Caller owns the DC and retains both selected objects.
        unsafe { SelectObject(dc, object) }
    }
    unsafe fn delete_bitmap(&self, bitmap: HBITMAP) -> i32 {
        // SAFETY: This is an acquired bitmap, after restoration was attempted.
        unsafe { DeleteObject(bitmap) }
    }
    unsafe fn delete_dc(&self, dc: HDC) -> i32 {
        // SAFETY: Caller owns this CreateCompatibleDC result.
        unsafe { DeleteDC(dc) }
    }
    unsafe fn release_dc(&self, hwnd: HWND, dc: HDC) -> i32 {
        // SAFETY: Caller supplies the original GetDC owner HWND and result.
        unsafe { ReleaseDC(hwnd, dc) }
    }
}

#[cfg(feature = "windows-gdi")]
// SAFETY: Direct Win32 calls preserve the checked spans and native ownership.
unsafe impl GdiApi for SystemGdi {
    unsafe fn copy(&self, destination: HDC, width: i32, height: i32, source: HDC) -> i32 {
        // SAFETY: Caller validates positive bitmap bounds and live DCs.
        unsafe { BitBlt(destination, 0, 0, width, height, source, 0, 0, SRCCOPY) }
    }
    unsafe fn object(&self, bitmap: HBITMAP, length: i32, output: *mut c_void) -> i32 {
        // SAFETY: Caller supplies an aligned writable BITMAP with its exact size.
        unsafe { GetObjectW(bitmap, length, output) }
    }
    unsafe fn read(&self, bitmap: HBITMAP, length: i32, output: *mut c_void) -> i32 {
        // SAFETY: Caller supplies exactly the checked writable byte span.
        unsafe { GetBitmapBits(bitmap, length, output) }
    }
}
