// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Original GetBitmapBits path shared by window GDI and ScreenDC. Conversion
// remains before native cleanup; this introduces no second full-frame copy.
use super::{gdi_api::GdiApi, gdi_resources::Resources, gdi_types::*, native::*};
use std::mem;

pub(super) fn convert<A: GdiApi, C: BgraConverter>(
    resources: &Resources<'_, A>,
    width: i32,
    height: i32,
    converter: &mut C,
) -> Result<C::Image, CaptureError<C::Error>> {
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
#[path = "gdi_read_tests.rs"]
mod tests;
