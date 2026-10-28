// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::{
    gdi_api::{DcApi, SystemGdi},
    native::*,
};
use std::ffi::c_void;

const ROOT_OWNER: u32 = {
    assert!(GA_ROOTOWNER >= 0);
    GA_ROOTOWNER as u32
};
const PRINT_FLAGS: u32 = {
    assert!(PW_CLIENTONLY >= 0 && PW_RENDERFULLCONTENT >= 0);
    (PW_CLIENTONLY | PW_RENDERFULLCONTENT) as u32
};
const DIB_COLORS: u32 = {
    assert!(DIB_RGB_COLORS >= 0);
    DIB_RGB_COLORS as u32
};

/// # Safety
/// DcApi ownership/thread requirements apply. Queries retain no pointers.
/// dibits may write only the requested, checked 32-bit span and BITMAPINFO;
/// its bitmap must be unselected. print synchronously lends the live DC to
/// the window owner and does not transfer its ownership to that window.
pub(super) unsafe trait PrintApi: DcApi {
    unsafe fn is_window(&self, hwnd: HWND) -> i32;
    unsafe fn root_owner(&self, hwnd: HWND) -> HWND;
    unsafe fn last_popup(&self, hwnd: HWND) -> HWND;
    unsafe fn visible(&self, hwnd: HWND) -> i32;
    unsafe fn origin(&self, hwnd: HWND, point: *mut POINT) -> i32;
    unsafe fn print(&self, hwnd: HWND, dc: HDC) -> i32;
    unsafe fn dibits(
        &self,
        dc: HDC,
        bitmap: HBITMAP,
        height: u32,
        pixels: *mut c_void,
        info: *mut BITMAPINFO,
    ) -> i32;
}

// SAFETY: Direct Win32 forwarding; ownership and writable spans are checked
// by the capture implementation before entering this native boundary.
unsafe impl PrintApi for SystemGdi {
    unsafe fn is_window(&self, hwnd: HWND) -> i32 {
        // SAFETY: Opaque HWND; Win32 validates it without dereferencing Rust data.
        unsafe { IsWindow(hwnd) }
    }
    unsafe fn root_owner(&self, hwnd: HWND) -> HWND {
        // SAFETY: Query only; no ownership of the returned HWND is acquired.
        unsafe { GetAncestor(hwnd, ROOT_OWNER) }
    }
    unsafe fn last_popup(&self, hwnd: HWND) -> HWND {
        // SAFETY: Query only; returned window remains owned by its source.
        unsafe { GetLastActivePopup(hwnd) }
    }
    unsafe fn visible(&self, hwnd: HWND) -> i32 {
        // SAFETY: The OS validates this non-owning handle.
        unsafe { IsWindowVisible(hwnd) }
    }
    unsafe fn origin(&self, hwnd: HWND, point: *mut POINT) -> i32 {
        // SAFETY: Caller supplies a writable, initialized POINT for this call.
        unsafe { ClientToScreen(hwnd, point) }
    }
    unsafe fn print(&self, hwnd: HWND, dc: HDC) -> i32 {
        // SAFETY: The caller retains the compatible DC/selected bitmap and stays
        // on its owning thread until this synchronous, non-interruptible return.
        unsafe { PrintWindow(hwnd, dc, PRINT_FLAGS) }
    }
    unsafe fn dibits(
        &self,
        dc: HDC,
        bitmap: HBITMAP,
        height: u32,
        pixels: *mut c_void,
        info: *mut BITMAPINFO,
    ) -> i32 {
        // SAFETY: Caller unselected bitmap and supplies the complete checked
        // top-down 32-bit buffer and initialized requested format description.
        unsafe { GetDIBits(dc, bitmap, 0, height, pixels, info, DIB_COLORS) }
    }
}
