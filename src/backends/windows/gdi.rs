// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from MaaFramework GdiScreencap.{h,cpp} and HwndUtils.hpp::window_size.
// Original source identity and applicable licence are in THIRD_PARTY_NOTICES.

use super::gdi_resources::Resources;
pub(super) use super::gdi_types::*;
use super::{
    gdi_api::{GdiApi, SystemGdi},
    native::*,
};
use std::{marker::PhantomData, mem, rc::Rc};

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

        let (stride, length, native_length) = checked_layout(width, height)?;
        let mut bitmap = BITMAP::default();
        let bitmap_size =
            i32::try_from(mem::size_of::<BITMAP>()).map_err(|_| CaptureError::SizeOverflow)?;
        // SAFETY: bitmap points to an aligned BITMAP of exactly the stated size.
        // This query validates the source's 32-bit packed-DDB assumption; it
        // neither changes pixels nor replaces the original GetBitmapBits path.
        if unsafe {
            resources.api.object(
                resources.bitmap,
                bitmap_size,
                (&mut bitmap as *mut BITMAP).cast(),
            )
        } != bitmap_size
        {
            return Err(CaptureError::Native(NativeFailure::last("GetObjectW")));
        }
        validate_bitmap(&bitmap, width, height, stride)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(length)
            .map_err(CaptureError::Allocation)?;
        // Capacity was acquired fallibly; resize cannot allocate. Define every
        // byte before the OS call, including on any short-read failure.
        pixels.resize(length, 0_u8);
        // SAFETY: Bitmap is live and pixels has exactly the checked writable span.
        let copied = unsafe {
            resources
                .api
                .read(resources.bitmap, native_length, pixels.as_mut_ptr().cast())
        };
        check_read(copied, native_length)?;
        // Preserve bgra_to_bgr's original point before resource cleanup. The
        // temporary CPU buffer is borrowed only during this synchronous call.
        converter
            .bgra_to_bgr(BgraView {
                pixels: &pixels,
                width,
                height,
                stride,
            })
            .map_err(CaptureError::Conversion)
    }
}

fn require_handle<T, E>(handle: *mut T, operation: &'static str) -> Result<(), CaptureError<E>> {
    if handle.is_null() || handle.addr() == usize::MAX {
        Err(CaptureError::Native(NativeFailure::last(operation)))
    } else {
        Ok(())
    }
}

fn checked_layout<E>(width: i32, height: i32) -> Result<(usize, usize, i32), CaptureError<E>> {
    if width <= 0 || height <= 0 {
        return Err(CaptureError::InvalidDimensions);
    }
    let width = usize::try_from(width).map_err(|_| CaptureError::SizeOverflow)?;
    let height = usize::try_from(height).map_err(|_| CaptureError::SizeOverflow)?;
    let stride = width.checked_mul(4).ok_or(CaptureError::SizeOverflow)?;
    let length = stride
        .checked_mul(height)
        .ok_or(CaptureError::SizeOverflow)?;
    let native_length = i32::try_from(length).map_err(|_| CaptureError::SizeOverflow)?;
    Ok((stride, length, native_length))
}

fn validate_bitmap<E>(
    bitmap: &BITMAP,
    width: i32,
    height: i32,
    stride: usize,
) -> Result<(), CaptureError<E>> {
    if bitmap.bmType != 0
        || bitmap.bmWidth != width
        || bitmap.bmHeight != height
        || bitmap.bmPlanes != 1
        || bitmap.bmBitsPixel != 32
        || usize::try_from(bitmap.bmWidthBytes).ok() != Some(stride)
    {
        return Err(CaptureError::UnsupportedBitmap);
    }
    Ok(())
}

fn check_read<E>(copied: i32, required: i32) -> Result<(), CaptureError<E>> {
    if copied == 0 {
        return Err(CaptureError::Native(NativeFailure::last("GetBitmapBits")));
    }
    if copied != required {
        return Err(CaptureError::IncompleteRead { copied, required });
    }
    Ok(())
}

#[cfg(test)]
#[path = "gdi_tests.rs"]
mod tests;
