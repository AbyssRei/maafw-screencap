// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from MaaFramework GdiScreencap.{h,cpp} and HwndUtils.hpp::window_size.
// Original source identity and applicable licence are in THIRD_PARTY_NOTICES.

use super::gdi_resources::{Resources, require_handle};
pub(super) use super::gdi_types::*;
use super::{
    gdi_api::{GdiApi, SystemGdi},
    native::*,
};
use std::{marker::PhantomData, rc::Rc};

pub(crate) struct Attempt<T, E> {
    pub image: Result<T, CaptureError<E>>,
    pub cleanup: Cleanup,
}

/// Non-owning HWND, synchronous caller execution, with no mobility assertion.
/// The owning manager/window lifetime is migrated separately in P1.
pub(crate) struct GdiCapture {
    hwnd: HWND,
    _thread: PhantomData<Rc<()>>,
}

impl GdiCapture {
    pub fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            _thread: PhantomData,
        }
    }

    pub fn capture<C: BgraConverter>(&mut self, converter: &mut C) -> Attempt<C::Image, C::Error> {
        self.capture_with_api(&SystemGdi, converter)
    }

    fn capture_with_api<A: GdiApi, C: BgraConverter>(
        &mut self,
        api: &A,
        converter: &mut C,
    ) -> Attempt<C::Image, C::Error> {
        let mut resources = Resources::new(self.hwnd, api);
        let image = self.capture_inner(&mut resources, converter);
        let cleanup = resources.release();
        Attempt { image, cleanup }
    }

    fn capture_inner<A: GdiApi, C: BgraConverter>(
        &self,
        resources: &mut Resources<'_, A>,
        converter: &mut C,
    ) -> Result<C::Image, CaptureError<C::Error>> {
        if self.hwnd.is_null() {
            return Err(CaptureError::NullWindow);
        }
        // SAFETY: HWND is an opaque OS handle, validated by these native calls.
        // Acquired DCs/bitmap stay on this caller thread until Resources releases
        // them. No reference into the window or native bitmap escapes.
        resources.dc = unsafe { resources.api.get_dc(self.hwnd) };
        require_handle(resources.dc, "GetDC")?;
        // SAFETY: dc was acquired above and has not been released.
        resources.memory_dc = unsafe { resources.api.create_dc(resources.dc) };
        require_handle(resources.memory_dc, "CreateCompatibleDC")?;

        // Keep window_size's query after compatible-DC acquisition as upstream.
        let mut rectangle = RECT::default();
        // SAFETY: Output is a valid writable RECT; the OS validates the HWND.
        if unsafe { resources.api.client_rect(self.hwnd, &mut rectangle) } == 0 {
            return Err(CaptureError::InvalidDimensions);
        }
        let width = rectangle
            .right
            .checked_sub(rectangle.left)
            .ok_or(CaptureError::SizeOverflow)?;
        let height = rectangle
            .bottom
            .checked_sub(rectangle.top)
            .ok_or(CaptureError::SizeOverflow)?;
        if width <= 0 || height <= 0 {
            return Err(CaptureError::InvalidDimensions);
        }
        // SAFETY: dc is live and dimensions are positive. Native allocation
        // failure is handled without assuming a bitmap was obtained.
        resources.bitmap = unsafe { resources.api.create_bitmap(resources.dc, width, height) };
        require_handle(resources.bitmap, "CreateCompatibleBitmap")?;
        // SAFETY: Both objects are live and owned by this thread. Retain the
        // previous selection for restoration before bitmap/DC destruction.
        resources.previous = unsafe { resources.api.select(resources.memory_dc, resources.bitmap) };
        require_handle(resources.previous, "SelectObject")?;
        // SAFETY: The selected bitmap has exactly the positive requested size.
        if unsafe {
            resources
                .api
                .copy(resources.memory_dc, width, height, resources.dc)
        } == 0
        {
            return Err(CaptureError::Native(NativeFailure::last("BitBlt")));
        }

        super::gdi_read::convert(resources, width, height, converter)
    }
}

#[cfg(test)]
#[path = "gdi_tests.rs"]
mod tests;
