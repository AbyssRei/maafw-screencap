// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Checked CPU ownership inside the original sample callback/lock lifetime.
use super::Error;
use objc2_core_media::CMSampleBuffer;
use objc2_core_video::{
    CVPixelBuffer, CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow,
    CVPixelBufferGetDataSize, CVPixelBufferGetHeight, CVPixelBufferGetPixelFormatType,
    CVPixelBufferGetWidth, CVPixelBufferIsPlanar, CVPixelBufferLockBaseAddress,
    CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress, kCVPixelFormatType_32BGRA,
};
use std::sync::{
    Arc,
    atomic::{AtomicI32, AtomicUsize, Ordering},
};

#[derive(Default)]
pub(crate) struct Evidence {
    pub locks: AtomicUsize,
    pub unlocks: AtomicUsize,
    pub copies: AtomicUsize,
    pub copied_bytes: AtomicUsize,
    pub conversions: AtomicUsize,
    pub unlock_error: AtomicI32,
}
#[derive(Clone)]
pub(crate) struct Sample {
    pub width: usize,
    pub height: usize,
    pub native_stride: usize,
    bytes: Arc<Vec<u8>>,
}
impl Sample {
    pub fn pixels(&self) -> &[u8] {
        &self.bytes
    }
}
#[derive(Clone, Copy, Debug)]
struct Layout {
    width: usize,
    height: usize,
    stride: usize,
    row: usize,
    span: usize,
    packed: usize,
}
impl Layout {
    fn checked(
        width: usize,
        height: usize,
        stride: usize,
        available: usize,
    ) -> Result<Self, Error> {
        if width == 0 || height == 0 || width > i32::MAX as usize || height > i32::MAX as usize {
            return Err(Error::new("ScreenCaptureKit dimensions", -22));
        }
        let row = width
            .checked_mul(4)
            .ok_or(Error::new("BGRA row size", -75))?;
        let span = (height - 1)
            .checked_mul(stride)
            .and_then(|last| last.checked_add(row))
            .ok_or(Error::new("BGRA readable span", -75))?;
        let packed = row
            .checked_mul(height)
            .ok_or(Error::new("BGRA CPU size", -75))?;
        if stride < row
            || span > available
            || span > isize::MAX as usize
            || packed > isize::MAX as usize
        {
            return Err(Error::new("ScreenCaptureKit short/oversized storage", -22));
        }
        Ok(Self {
            width,
            height,
            stride,
            row,
            span,
            packed,
        })
    }
}
struct Lock<'a> {
    image: &'a CVPixelBuffer,
    evidence: &'a Evidence,
}
impl Drop for Lock<'_> {
    fn drop(&mut self) {
        // SAFETY: The retained image and its successful read-only lock outlive
        // this guard. All native pixel borrows and actual conversion end first.
        let code =
            unsafe { CVPixelBufferUnlockBaseAddress(self.image, CVPixelBufferLockFlags::ReadOnly) };
        self.evidence.unlock_error.store(code, Ordering::Release);
        self.evidence.unlocks.store(1, Ordering::Release);
    }
}
/// The caller's actual conversion stays inside the source lock/callback. This
/// helper copies initialized target rows first, adding explicit work, and never
/// passes the native image/base pointer or an unbounded native lease outward.
pub(crate) fn with_sample<T>(
    sample: &CMSampleBuffer,
    evidence: &Evidence,
    convert: impl FnOnce(Option<Sample>) -> Result<T, Error>,
) -> Result<T, Error> {
    // SAFETY: This is the native completion's live sample; retain its image
    // before reading. A missing image remains the source's engaged empty Mat.
    let Some(image) = (unsafe { sample.image_buffer() }) else {
        return convert(None);
    };
    if CVPixelBufferIsPlanar(&image)
        || CVPixelBufferGetPixelFormatType(&image) != kCVPixelFormatType_32BGRA
    {
        return Err(Error::new("ScreenCaptureKit non-BGRA/planar sample", -22));
    }
    // SAFETY: Retained native image, original read-only access request. Unlike
    // the source's ignored failure, unsafe CPU access after lock failure is
    // rejected as an explicit safety adaptation.
    let code = unsafe { CVPixelBufferLockBaseAddress(&image, CVPixelBufferLockFlags::ReadOnly) };
    if code != 0 {
        return Err(Error::new("CVPixelBufferLockBaseAddress", i64::from(code)));
    }
    evidence.locks.store(1, Ordering::Release);
    let lock = Lock {
        image: &image,
        evidence,
    };
    let width = CVPixelBufferGetWidth(&image);
    let height = CVPixelBufferGetHeight(&image);
    let pointer = CVPixelBufferGetBaseAddress(&image).cast::<u8>();
    if pointer.is_null() || width == 0 || height == 0 {
        let result = convert(None);
        drop(lock);
        return result;
    }
    let layout = Layout::checked(
        width,
        height,
        CVPixelBufferGetBytesPerRow(&image),
        CVPixelBufferGetDataSize(&image),
    )?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(layout.packed)
        .map_err(|_| Error::allocation())?;
    for y in 0..layout.height {
        let offset = y
            .checked_mul(layout.stride)
            .ok_or(Error::new("BGRA row offset", -75))?;
        debug_assert!(
            offset
                .checked_add(layout.row)
                .is_some_and(|end| end <= layout.span)
        );
        // SAFETY: Successful native read lock, checked initialized target row
        // and complete readable span. Padding is never copied or exposed.
        bytes.extend_from_slice(unsafe {
            std::slice::from_raw_parts(pointer.add(offset), layout.row)
        });
    }
    let bytes = Arc::try_new(bytes).map_err(|_| Error::allocation())?;
    evidence.copies.store(1, Ordering::Release);
    evidence
        .copied_bytes
        .store(layout.packed, Ordering::Release);
    evidence.conversions.store(1, Ordering::Release);
    let result = convert(Some(Sample {
        width: layout.width,
        height: layout.height,
        native_stride: layout.stride,
        bytes,
    }));
    drop(lock);
    // U ignores Unlock's return. Preserve the primary conversion/result and
    // retain a distinct cleanup code; do not introduce a P3 success rejection.
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_overflowing_or_unreadable_bgra_layouts() {
        assert!(Layout::checked(47, 31, 188, 188 * 31).is_ok());
        for (width, height, stride, span) in [
            (0, 31, 188, 188 * 31),
            (47, usize::MAX, 188, usize::MAX),
            (47, 31, 187, usize::MAX),
            (47, 31, usize::MAX, usize::MAX),
            (47, 31, 188, 188 * 31 - 1),
        ] {
            assert!(Layout::checked(width, height, stride, span).is_err());
        }
    }
}
