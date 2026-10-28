// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::{
    gdi_api::DcApi,
    gdi_types::{Cleanup, NativeFailure},
    native::*,
};
use std::{mem, ptr::null_mut};

pub(super) struct Resources<'a, A: DcApi> {
    pub(super) api: &'a A,
    pub(super) hwnd: HWND,
    pub(super) dc: HDC,
    pub(super) memory_dc: HDC,
    pub(super) bitmap: HBITMAP,
    pub(super) previous: HGDIOBJ,
}

impl<'a, A: DcApi> Resources<'a, A> {
    pub(super) fn new(hwnd: HWND, api: &'a A) -> Self {
        Self {
            api,
            hwnd,
            dc: null_mut(),
            memory_dc: null_mut(),
            bitmap: null_mut(),
            previous: null_mut(),
        }
    }

    pub(super) fn release(&mut self) -> Cleanup {
        let mut errors = Cleanup::default();
        // Consume each cleanup responsibility before calling the native API.
        // A failed attempt is reported, never silently retried by Drop.
        let previous = mem::replace(&mut self.previous, null_mut());
        if !previous.is_null() && previous.addr() != usize::MAX {
            // SAFETY: Successful selection retained the original object and DC.
            let restored = unsafe { self.api.select(self.memory_dc, previous) };
            if restored.is_null() || restored.addr() == usize::MAX {
                errors.0[0] = Some(NativeFailure::last("SelectObject.restore"));
            }
        }
        let bitmap = mem::replace(&mut self.bitmap, null_mut());
        if !bitmap.is_null() {
            // SAFETY: This is the acquired bitmap; restoration was attempted first.
            if unsafe { self.api.delete_bitmap(bitmap) } == 0 {
                errors.0[1] = Some(NativeFailure::last("DeleteObject"));
            }
        }
        let dc = mem::replace(&mut self.memory_dc, null_mut());
        if !dc.is_null() {
            // SAFETY: This DC was created by CreateCompatibleDC on this thread.
            if unsafe { self.api.delete_dc(dc) } == 0 {
                errors.0[2] = Some(NativeFailure::last("DeleteDC"));
            }
        }
        let dc = mem::replace(&mut self.dc, null_mut());
        if !dc.is_null() {
            // SAFETY: Match the GetDC owner HWND, including native failure paths.
            if unsafe { self.api.release_dc(self.hwnd, dc) } == 0 {
                errors.0[3] = Some(NativeFailure::last("ReleaseDC"));
            }
        }
        errors
    }
}

impl<A: DcApi> Drop for Resources<'_, A> {
    fn drop(&mut self) {
        // Unwind fallback never panics or allocates. Ordinary returns expose
        // every cleanup failure separately in Attempt::cleanup.
        let _ = self.release();
    }
}
