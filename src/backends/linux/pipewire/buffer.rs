// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Checked target-row ownership for the fixed source's requested-frame copy.
//! Mapping, DMA synchronization and native buffer return remain the driver's
//! responsibilities. No native pointer/lease is retained by a published frame.
use super::Format;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CopyError {
    UnsupportedFormat,
    InvalidDimensions,
    NegativeStride,
    ShortStride,
    NonzeroOffset,
    CorruptChunk,
    ShortSpan,
    SizeOverflow,
    NullPointer,
    Allocation,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Chunk {
    pub stride: i32,
    pub offset: u32,
    pub size: u32,
    pub flags: i32,
}

/// Constructed before mapping or making any pixel reference. U copies from
/// offset zero; accepting nonzero offsets would silently change that behavior.
/// Such layouts are explicitly rejected pending the separately planned P3 work.
#[derive(Clone, Copy, Debug)]
pub(super) struct Layout {
    width: u32,
    height: u32,
    format: Format,
    row: usize,
    stride: usize,
    rows: usize,
    span: usize,
    length: usize,
}
impl Layout {
    pub fn checked(
        width: u32,
        height: u32,
        format: Format,
        maxsize: u32,
        chunk: Option<Chunk>,
    ) -> Result<Self, CopyError> {
        let channels = format.channels().ok_or(CopyError::UnsupportedFormat)?;
        if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32 {
            return Err(CopyError::InvalidDimensions);
        }
        let width_usize = usize::try_from(width).map_err(|_| CopyError::SizeOverflow)?;
        let rows = usize::try_from(height).map_err(|_| CopyError::SizeOverflow)?;
        let row = width_usize
            .checked_mul(channels)
            .ok_or(CopyError::SizeOverflow)?;
        // The original row_bytes is an int and OpenCV dimensions/step must fit
        // that path. Reject its undefined signed-overflow domain explicitly.
        i32::try_from(row).map_err(|_| CopyError::SizeOverflow)?;
        let stride = match chunk {
            Some(chunk) => {
                if chunk.offset != 0 {
                    return Err(CopyError::NonzeroOffset);
                }
                if chunk.flags != 0 {
                    return Err(CopyError::CorruptChunk);
                }
                if chunk.stride < 0 {
                    return Err(CopyError::NegativeStride);
                }
                if chunk.stride == 0 {
                    row
                } else {
                    usize::try_from(chunk.stride).map_err(|_| CopyError::SizeOverflow)?
                }
            }
            None => row,
        };
        if stride < row {
            return Err(CopyError::ShortStride);
        }
        let span = (rows - 1)
            .checked_mul(stride)
            .and_then(|last| last.checked_add(row))
            .ok_or(CopyError::SizeOverflow)?;
        let length = row.checked_mul(rows).ok_or(CopyError::SizeOverflow)?;
        isize::try_from(span).map_err(|_| CopyError::SizeOverflow)?;
        isize::try_from(length).map_err(|_| CopyError::SizeOverflow)?;
        if span > usize::try_from(maxsize).map_err(|_| CopyError::SizeOverflow)?
            || chunk.is_some_and(|chunk| {
                u64::try_from(span).unwrap_or(u64::MAX) > u64::from(chunk.size)
            })
        {
            return Err(CopyError::ShortSpan);
        }
        Ok(Self {
            width,
            height,
            format,
            row,
            stride,
            rows,
            span,
            length,
        })
    }

    /// Bytes that an owned mapping must make readable. This excludes trailing
    /// padding in the final row, which is never read or exposed.
    pub fn readable_span(self) -> usize {
        self.span
    }
    pub fn mapping_length(self) -> Result<usize, CopyError> {
        let length = self
            .stride
            .checked_mul(self.rows)
            .ok_or(CopyError::SizeOverflow)?;
        isize::try_from(length).map_err(|_| CopyError::SizeOverflow)?;
        Ok(length)
    }
}

/// All bytes are initialized target pixels, in source row/channel order.
/// Cloning retains CPU storage only, without prolonging native buffer leases.
#[derive(Clone, Debug)]
pub(super) struct RawFrame {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub stride: usize,
    pixels: Arc<Vec<u8>>,
}
impl RawFrame {
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CopyEvidence {
    pub allocated_storage: bool,
    pub copied_bytes: usize,
}

#[derive(Default)]
pub(super) struct RawCache {
    latest: Option<RawFrame>,
}
impl RawCache {
    pub fn snapshot(&self) -> Option<RawFrame> {
        self.latest.clone()
    }
    pub fn is_empty(&self) -> bool {
        self.latest.is_none()
    }
    pub fn clear(&mut self) {
        self.latest = None;
    }

    /// Copy only while the owning native dequeue/map/sync lease is valid.
    ///
    /// # Safety
    /// If non-null, `source` must point to one allocation/mapping containing the validated
    /// readable span; every target-row byte must be initialized and CPU readable
    /// with required native synchronization complete. It must not alias this
    /// cache's CPU storage. The producer must not write or revoke it during the
    /// call. Non-target padding is neither read nor formed into a Rust slice.
    /// All references created here end before the native lease can be returned.
    /// Null is rejected without dereferencing it.
    pub unsafe fn copy_native(
        &mut self,
        source: *const u8,
        layout: Layout,
    ) -> Result<CopyEvidence, CopyError> {
        if source.is_null() {
            return Err(CopyError::NullPointer);
        }
        // Arc::get_mut checks both strong and weak ownership. In particular a
        // retained BGR result must not observe later native writes, unlike the
        // original Mat::create cache reuse. This is an explicit safety adaptation:
        // new storage when shared, one target-row copy, no second full-frame copy.
        let reusable = self.latest.as_mut().and_then(|frame| {
            if frame.width == layout.width
                && frame.height == layout.height
                && frame.format == layout.format
            {
                Arc::get_mut(&mut frame.pixels)
            } else {
                None
            }
        });
        if let Some(pixels) = reusable {
            // Same immutable layout and unique owner establish exact length.
            // SAFETY: Valid native lease is required by this function; layout
            // validated every row and source does not alias this unique storage.
            unsafe { copy_rows(source, layout, pixels) };
            return Ok(CopyEvidence {
                allocated_storage: false,
                copied_bytes: layout.length,
            });
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(layout.length)
            .map_err(|_| CopyError::Allocation)?;
        pixels.resize(layout.length, 0);
        // Acquire all fallible owners before copying or replacing the old cache.
        // A failed allocation leaves its previous sample and pixels intact.
        let mut pixels = Arc::try_new(pixels).map_err(|_| CopyError::Allocation)?;
        let unique = Arc::get_mut(&mut pixels).ok_or(CopyError::Allocation)?;
        // SAFETY: The native source lease outlives the call; newly allocated CPU
        // storage cannot alias it and has the exact checked initialized length.
        unsafe { copy_rows(source, layout, unique) };
        self.latest = Some(RawFrame {
            width: layout.width,
            height: layout.height,
            format: layout.format,
            stride: layout.row,
            pixels,
        });
        Ok(CopyEvidence {
            allocated_storage: true,
            copied_bytes: layout.length,
        })
    }
}

unsafe fn copy_rows(source: *const u8, layout: Layout, destination: &mut [u8]) {
    for y in 0..layout.rows {
        // All products/additions are bounded by checked span/length. Layout
        // fields are private, so an unchecked instance cannot enter this loop.
        let offset = y * layout.stride;
        let out = y * layout.row;
        // SAFETY: Caller owns the readable native rows for this synchronous
        // copy; no reference to padding or bytes outside the target is made.
        let row = unsafe { std::slice::from_raw_parts(source.add(offset), layout.row) };
        destination[out..out + layout.row].copy_from_slice(row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(stride: i32, size: u32) -> Result<Layout, CopyError> {
        Layout::checked(
            2,
            2,
            Format::Bgr,
            size,
            Some(Chunk {
                stride,
                size,
                offset: 0,
                flags: 0,
            }),
        )
    }
    fn copy(cache: &mut RawCache, bytes: &[u8], layout: Layout) -> CopyEvidence {
        assert!(layout.readable_span() <= bytes.len());
        // SAFETY: Test owns all initialized source rows, length was checked and
        // cache storage is separately allocated and cannot alias this input.
        unsafe { cache.copy_native(bytes.as_ptr(), layout) }.unwrap()
    }

    #[test]
    fn copies_only_target_rows_and_reuses_only_unique_storage() {
        let padded = [1, 2, 3, 4, 5, 6, 0xa5, 0xa5, 7, 8, 9, 10, 11, 12];
        let layout = layout(8, 14).unwrap();
        let mut cache = RawCache::default();
        assert_eq!(copy(&mut cache, &padded, layout).copied_bytes, 12);
        let second = copy(&mut cache, &padded, layout);
        assert!(!second.allocated_storage);
        let held = cache.snapshot().unwrap();
        assert_eq!(held.pixels(), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        assert_eq!(held.stride, 6);
        let different = [42; 14];
        assert!(copy(&mut cache, &different, layout).allocated_storage);
        assert!(!Arc::ptr_eq(
            &held.pixels,
            &cache.latest.as_ref().unwrap().pixels
        ));
        assert_eq!(held.pixels(), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        cache.clear();
        assert!(cache.is_empty());
        let reader = std::thread::spawn(move || held.pixels().to_vec());
        assert_eq!(
            reader.join().unwrap(),
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
        );
    }

    #[test]
    fn rejects_native_layouts_outside_the_checked_copy_domain() {
        assert_eq!(layout(-8, 16).unwrap_err(), CopyError::NegativeStride);
        assert_eq!(layout(5, 16).unwrap_err(), CopyError::ShortStride);
        assert_eq!(layout(8, 13).unwrap_err(), CopyError::ShortSpan);
        assert_eq!(layout(0, 11).unwrap_err(), CopyError::ShortSpan);
        for (offset, flags, expected) in [
            (1, 0, CopyError::NonzeroOffset),
            (0, 1, CopyError::CorruptChunk),
        ] {
            assert_eq!(
                Layout::checked(
                    2,
                    2,
                    Format::Bgr,
                    16,
                    Some(Chunk {
                        stride: 8,
                        size: 16,
                        offset,
                        flags
                    })
                )
                .unwrap_err(),
                expected
            );
        }
        assert_eq!(
            Layout::checked(0, 2, Format::Bgr, 16, None).unwrap_err(),
            CopyError::InvalidDimensions
        );
        assert_eq!(
            Layout::checked(u32::MAX, 2, Format::Bgr, 16, None).unwrap_err(),
            CopyError::InvalidDimensions
        );
        assert_eq!(
            Layout::checked(i32::MAX as u32, 2, Format::Bgr, u32::MAX, None).unwrap_err(),
            CopyError::SizeOverflow
        );
        assert_eq!(
            Layout::checked(2, 2, Format::Unknown, 16, None).unwrap_err(),
            CopyError::UnsupportedFormat
        );
    }

    #[test]
    fn keeps_source_row_order_and_format_while_excluding_padding() {
        for format in [
            Format::Bgra,
            Format::Bgrx,
            Format::Rgba,
            Format::Rgbx,
            Format::Rgb,
            Format::Bgr,
            Format::Yuy2,
        ] {
            let row = format.channels().unwrap() * 3;
            let stride = row + 3;
            let span = stride + row;
            let layout = Layout::checked(
                3,
                2,
                format,
                u32::try_from(span).unwrap(),
                Some(Chunk {
                    stride: i32::try_from(stride).unwrap(),
                    offset: 0,
                    size: u32::try_from(span).unwrap(),
                    flags: 0,
                }),
            )
            .unwrap();
            let mut source = vec![0xa5; span];
            for y in 0..2 {
                for x in 0..row {
                    source[y * stride + x] = u8::try_from(y * row + x).unwrap();
                }
            }
            let mut cache = RawCache::default();
            copy(&mut cache, &source, layout);
            let frame = cache.snapshot().unwrap();
            assert_eq!(frame.format, format);
            assert_eq!(
                frame.pixels(),
                (0..2 * row)
                    .map(|i| u8::try_from(i).unwrap())
                    .collect::<Vec<_>>()
            );
        }
        assert_eq!(layout(0, 12).unwrap().stride, 6);
        assert_eq!(
            Layout::checked(2, 2, Format::Bgr, 12, None).unwrap().stride,
            6
        );
    }

    #[test]
    fn rejects_null_without_replacing_the_previous_sample() {
        let layout = layout(0, 12).unwrap();
        let mut cache = RawCache::default();
        copy(&mut cache, &[7; 12], layout);
        let held = cache.snapshot().unwrap();
        // SAFETY: Null is rejected before any dereference; this intentionally
        // exercises the explicit check, without claiming null is readable.
        assert_eq!(
            unsafe { cache.copy_native(std::ptr::null(), layout) },
            Err(CopyError::NullPointer)
        );
        assert!(Arc::ptr_eq(
            &held.pixels,
            &cache.latest.as_ref().unwrap().pixels
        ));
        assert_eq!(held.pixels(), [7; 12]);
    }
}
