// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from MaaFramework GdiScreencap.{h,cpp} and HwndUtils.hpp::window_size.
// Original source identity and applicable licence are in THIRD_PARTY_NOTICES.

use super::native::*;
use std::{collections::TryReserveError, marker::PhantomData, mem, ptr::null_mut, rc::Rc};

/// P1's private conversion boundary. It runs before the source GDI cleanup.
/// The converter must own its result independently of this borrowed input.
pub(crate) trait BgraConverter {
    type Image;
    type Error;
    fn bgra_to_bgr(&mut self, input: BgraView<'_>) -> Result<Self::Image, Self::Error>;
}

pub(crate) struct BgraView<'a> {
    pub pixels: &'a [u8],
    pub width: i32,
    pub height: i32,
    pub stride: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeFailure {
    pub operation: &'static str,
    // Not every GDI API defines GetLastError. Preserve the observed value
    // without presenting it as a guaranteed API-specific error code.
    pub last_error: u32,
}

impl NativeFailure {
    fn last(operation: &'static str) -> Self {
        Self {
            operation,
            // SAFETY: GetLastError has no pointer/ownership preconditions.
            last_error: unsafe { GetLastError() },
        }
    }
}

#[derive(Debug)]
pub(crate) enum CaptureError<E> {
    NullWindow,
    Native(NativeFailure),
    InvalidDimensions,
    SizeOverflow,
    UnsupportedBitmap,
    IncompleteRead { copied: i32, required: i32 },
    Allocation(TryReserveError),
    Conversion(E),
}

/// Independent evidence, rather than overwriting a primary capture error.
/// Slots follow the original restore/delete-bitmap/delete-DC/release-DC order.
#[derive(Debug, Default)]
pub(crate) struct Cleanup(pub [Option<NativeFailure>; 4]);

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
        let mut resources = Resources::new(self.hwnd);
        let image = self.capture_inner(&mut resources, converter);
        let cleanup = resources.release();
        Attempt { image, cleanup }
    }

    fn capture_inner<C: BgraConverter>(
        &self,
        resources: &mut Resources,
        converter: &mut C,
    ) -> Result<C::Image, CaptureError<C::Error>> {
        if self.hwnd.is_null() {
            return Err(CaptureError::NullWindow);
        }
        // SAFETY: HWND is an opaque OS handle, validated by these native calls.
        // Acquired DCs/bitmap stay on this caller thread until Resources releases
        // them. No reference into the window or native bitmap escapes.
        resources.dc = unsafe { GetDC(self.hwnd) };
        require_handle(resources.dc, "GetDC")?;
        // SAFETY: dc was acquired above and has not been released.
        resources.memory_dc = unsafe { CreateCompatibleDC(resources.dc) };
        require_handle(resources.memory_dc, "CreateCompatibleDC")?;

        // Keep window_size's query after compatible-DC acquisition as upstream.
        let mut rectangle = RECT::default();
        // SAFETY: Output is a valid writable RECT; the OS validates the HWND.
        if unsafe { GetClientRect(self.hwnd, &mut rectangle) } == 0 {
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
        resources.bitmap = unsafe { CreateCompatibleBitmap(resources.dc, width, height) };
        require_handle(resources.bitmap, "CreateCompatibleBitmap")?;
        // SAFETY: Both objects are live and owned by this thread. Retain the
        // previous selection for restoration before bitmap/DC destruction.
        resources.previous = unsafe { SelectObject(resources.memory_dc, resources.bitmap) };
        require_handle(resources.previous, "SelectObject")?;
        // SAFETY: The selected bitmap has exactly the positive requested size.
        if unsafe {
            BitBlt(
                resources.memory_dc,
                0,
                0,
                width,
                height,
                resources.dc,
                0,
                0,
                SRCCOPY,
            )
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
            GetObjectW(
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
        let copied =
            unsafe { GetBitmapBits(resources.bitmap, native_length, pixels.as_mut_ptr().cast()) };
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

struct Resources {
    hwnd: HWND,
    dc: HDC,
    memory_dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
}

impl Resources {
    fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            dc: null_mut(),
            memory_dc: null_mut(),
            bitmap: null_mut(),
            previous: null_mut(),
        }
    }

    fn release(&mut self) -> Cleanup {
        let mut errors = Cleanup::default();
        // Consume each cleanup responsibility before calling the native API.
        // A failed attempt is reported, never silently retried by Drop.
        let previous = mem::replace(&mut self.previous, null_mut());
        if !previous.is_null() && previous.addr() != usize::MAX {
            // SAFETY: Successful selection retained the original object and DC.
            let restored = unsafe { SelectObject(self.memory_dc, previous) };
            if restored.is_null() || restored.addr() == usize::MAX {
                errors.0[0] = Some(NativeFailure::last("SelectObject.restore"));
            }
        }
        let bitmap = mem::replace(&mut self.bitmap, null_mut());
        if !bitmap.is_null() {
            // SAFETY: This is the acquired bitmap; restoration was attempted first.
            if unsafe { DeleteObject(bitmap) } == 0 {
                errors.0[1] = Some(NativeFailure::last("DeleteObject"));
            }
        }
        let dc = mem::replace(&mut self.memory_dc, null_mut());
        if !dc.is_null() {
            // SAFETY: This DC was created by CreateCompatibleDC on this thread.
            if unsafe { DeleteDC(dc) } == 0 {
                errors.0[2] = Some(NativeFailure::last("DeleteDC"));
            }
        }
        let dc = mem::replace(&mut self.dc, null_mut());
        if !dc.is_null() {
            // SAFETY: Match the GetDC owner HWND, including native failure paths.
            if unsafe { ReleaseDC(self.hwnd, dc) } == 0 {
                errors.0[3] = Some(NativeFailure::last("ReleaseDC"));
            }
        }
        errors
    }
}

impl Drop for Resources {
    fn drop(&mut self) {
        // Unwind fallback never panics or allocates. Ordinary returns expose
        // every cleanup failure separately in Attempt::cleanup.
        let _ = self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_dimensions_and_lengths_that_cannot_reach_getbitmapbits() {
        for (width, height) in [(0, 1), (1, 0), (-1, 2), (i32::MAX, 1), (32768, 32768)] {
            assert!(checked_layout::<()>(width, height).is_err());
        }
        assert_eq!(checked_layout::<()>(37, 23).unwrap(), (148, 3404, 3404));
    }

    #[test]
    fn rejects_short_reads_and_incompatible_device_bitmap_layouts() {
        assert!(matches!(
            check_read::<()>(7, 8),
            Err(CaptureError::IncompleteRead {
                copied: 7,
                required: 8
            })
        ));
        assert!(check_read::<()>(-1, 8).is_err());
        let mut bitmap = BITMAP {
            bmWidth: 37,
            bmHeight: 23,
            bmWidthBytes: 148,
            bmPlanes: 1,
            bmBitsPixel: 32,
            ..Default::default()
        };
        assert!(validate_bitmap::<()>(&bitmap, 37, 23, 148).is_ok());
        bitmap.bmBitsPixel = 24;
        assert!(matches!(
            validate_bitmap::<()>(&bitmap, 37, 23, 148),
            Err(CaptureError::UnsupportedBitmap)
        ));
        bitmap.bmBitsPixel = 32;
        bitmap.bmWidthBytes = 152;
        assert!(validate_bitmap::<()>(&bitmap, 37, 23, 148).is_err());
    }

    #[cfg(feature = "prototype-windows")]
    mod parity {
        include!("gdi_parity_tests.rs");
    }
}
