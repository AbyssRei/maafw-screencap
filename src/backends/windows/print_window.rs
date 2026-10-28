// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from fixed MaaFramework PrintWindowScreencap.{h,cpp}; see notices.
use super::{
    gdi_api::SystemGdi, gdi_resources::Resources, gdi_types::*, native::*,
    print_window_api::PrintApi,
};
use std::{marker::PhantomData, rc::Rc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Overlay {
    pub source_x: i32,
    pub source_y: i32,
    pub destination_x: i32,
    pub destination_y: i32,
    pub width: i32,
    pub height: i32,
}

/// P1 only: source OpenCV ROI copyTo occurs after both captures/conversions
/// and their native cleanup. Implementations must not retain borrowed images.
pub(crate) trait PrintConverter: BgraConverter {
    fn overlay(
        &mut self,
        destination: &mut Self::Image,
        source: &Self::Image,
        rectangle: Overlay,
    ) -> Result<(), Self::Error>;
}

pub(crate) struct Attempt<T, E> {
    pub image: Result<T, CaptureError<E>>,
    pub main_cleanup: Cleanup,
    pub popup_cleanup: Cleanup,
    pub popup_failure: Option<CaptureError<E>>,
}

struct WindowImage<T> {
    image: T,
    width: i32,
    height: i32,
}

pub(crate) struct PrintWindowCapture {
    hwnd: HWND,
    _thread: PhantomData<Rc<()>>,
}

impl PrintWindowCapture {
    pub fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            _thread: PhantomData,
        }
    }

    pub fn capture<C: PrintConverter>(&mut self, converter: &mut C) -> Attempt<C::Image, C::Error> {
        self.capture_with_api(&SystemGdi, converter)
    }

    fn capture_with_api<A: PrintApi, C: PrintConverter>(
        &self,
        api: &A,
        converter: &mut C,
    ) -> Attempt<C::Image, C::Error> {
        let mut main_cleanup = Cleanup::default();
        let mut popup_cleanup = Cleanup::default();
        let mut popup_failure = None;
        let image = self.capture_inner(
            api,
            converter,
            &mut main_cleanup,
            &mut popup_cleanup,
            &mut popup_failure,
        );
        Attempt {
            image,
            main_cleanup,
            popup_cleanup,
            popup_failure,
        }
    }

    fn capture_inner<A: PrintApi, C: PrintConverter>(
        &self,
        api: &A,
        converter: &mut C,
        main_cleanup: &mut Cleanup,
        popup_cleanup: &mut Cleanup,
        popup_failure: &mut Option<CaptureError<C::Error>>,
    ) -> Result<C::Image, CaptureError<C::Error>> {
        if self.hwnd.is_null() {
            return Err(CaptureError::NullWindow);
        }
        let mut main = capture_window(api, self.hwnd, converter, main_cleanup)?;
        // SAFETY: Non-owning HWND queries; preserve the original root fallback.
        let root = unsafe { api.root_owner(self.hwnd) };
        let root = if root.is_null() { self.hwnd } else { root };
        // SAFETY: Query only, without retaining a native window object.
        let popup = unsafe { api.last_popup(root) };
        if popup.is_null() || popup == self.hwnd || popup == root
            // SAFETY: Short-circuited source eligibility queries validate HWNDs.
            || unsafe { api.is_window(popup) } == 0
            || unsafe { api.visible(popup) } == 0
        {
            return Ok(main.image);
        }
        let popup = match capture_window(api, popup, converter, popup_cleanup) {
            Ok(image) => (popup, image),
            Err(
                error @ (CaptureError::NullWindow
                | CaptureError::InvalidWindow
                | CaptureError::InvalidDimensions
                | CaptureError::Native(_)
                | CaptureError::IncompleteScanlines { .. }),
            ) => {
                *popup_failure = Some(error);
                return Ok(main.image);
            }
            // Conversion/allocation exceptions escape the source's optional
            // fallback; preserve that whole-operation failure distinction.
            Err(error) => return Err(error),
        };
        let mut main_origin = POINT::default();
        let mut popup_origin = POINT::default();
        // SAFETY: Valid initialized POINT outputs. Both captures have released
        // their native resources before these original origin-query points.
        if unsafe { api.origin(self.hwnd, &mut main_origin) } == 0
            || unsafe { api.origin(popup.0, &mut popup_origin) } == 0
        {
            *popup_failure = Some(CaptureError::Native(NativeFailure::last("ClientToScreen")));
            return Ok(main.image);
        }
        if let Some(rectangle) = intersection::<C::Error>(
            (main.width, main.height),
            (popup.1.width, popup.1.height),
            (
                i64::from(popup_origin.x) - i64::from(main_origin.x),
                i64::from(popup_origin.y) - i64::from(main_origin.y),
            ),
        )? {
            converter
                .overlay(&mut main.image, &popup.1.image, rectangle)
                .map_err(CaptureError::Conversion)?;
        }
        Ok(main.image)
    }
}

fn capture_window<A: PrintApi, C: PrintConverter>(
    api: &A,
    hwnd: HWND,
    converter: &mut C,
    cleanup: &mut Cleanup,
) -> Result<WindowImage<C::Image>, CaptureError<C::Error>> {
    let mut resources = Resources::new(hwnd, api);
    let image = capture_window_inner(&mut resources, converter);
    *cleanup = resources.release();
    image
}

fn capture_window_inner<A: PrintApi, C: PrintConverter>(
    resources: &mut Resources<'_, A>,
    converter: &mut C,
) -> Result<WindowImage<C::Image>, CaptureError<C::Error>> {
    let api = resources.api;
    // SAFETY: IsWindow validates the non-owning handle before DC acquisition.
    if resources.hwnd.is_null() || unsafe { api.is_window(resources.hwnd) } == 0 {
        return Err(CaptureError::InvalidWindow);
    }
    let mut rectangle = RECT::default();
    // SAFETY: Valid writable RECT. Preserve source query before native resources.
    if unsafe { api.client_rect(resources.hwnd, &mut rectangle) } == 0 {
        return Err(CaptureError::Native(NativeFailure::last("GetClientRect")));
    }
    let width = rectangle
        .right
        .checked_sub(rectangle.left)
        .ok_or(CaptureError::SizeOverflow)?;
    let height = rectangle
        .bottom
        .checked_sub(rectangle.top)
        .ok_or(CaptureError::SizeOverflow)?;
    let (stride, length) = layout::<C::Error>(width, height)?;
    // SAFETY: These DCs/bitmap remain owned on this caller thread through all
    // synchronous native calls, conversion and Resources cleanup.
    unsafe {
        resources.dc = api.get_dc(resources.hwnd);
        handle(resources.dc, "GetDC")?;
        resources.memory_dc = api.create_dc(resources.dc);
        handle(resources.memory_dc, "CreateCompatibleDC")?;
        resources.bitmap = api.create_bitmap(resources.dc, width, height);
        handle(resources.bitmap, "CreateCompatibleBitmap")?;
        resources.previous = api.select(resources.memory_dc, resources.bitmap);
        handle(resources.previous, "SelectObject")?;
        if api.print(resources.hwnd, resources.memory_dc) == 0 {
            return Err(CaptureError::Native(NativeFailure::last("PrintWindow")));
        }
    }
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(size_of::<BITMAPINFOHEADER>())
                .map_err(|_| CaptureError::SizeOverflow)?,
            biWidth: width,
            biHeight: height.checked_neg().ok_or(CaptureError::SizeOverflow)?,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: u32::try_from(BI_RGB).map_err(|_| CaptureError::SizeOverflow)?,
            ..Default::default()
        },
        ..Default::default()
    };
    // A u32 owner guarantees DWORD-aligned rows even when a consumer installs
    // a global allocator that only honors the requested allocation alignment.
    // layout proved length == width * height * 4 with initialized whole pixels.
    let words = length / size_of::<u32>();
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(words)
        .map_err(CaptureError::Allocation)?;
    pixels.resize(words, 0_u32);
    // SAFETY: Explicit safety adaptation: GetDIBits requires an unselected
    // bitmap. Retain the original final restoration/cleanup responsibilities.
    let unselected = unsafe { api.unselect_for_read(resources.memory_dc, resources.previous) };
    handle(unselected, "SelectObject.before-GetDIBits")?;
    // SAFETY: Complete initialized top-down 32-bit span; positive height fits
    // u32, and the bitmap is now absent from its exclusively owned memory DC.
    let copied = unsafe {
        api.dibits(
            resources.memory_dc,
            resources.bitmap,
            u32::try_from(height).map_err(|_| CaptureError::SizeOverflow)?,
            pixels.as_mut_ptr().cast(),
            &mut info,
        )
    };
    if copied == 0 {
        return Err(CaptureError::Native(NativeFailure::last("GetDIBits")));
    }
    if copied != height {
        return Err(CaptureError::IncompleteScanlines {
            copied,
            required: height,
        });
    }
    // SAFETY: Restore the source's selected-bitmap state before conversion.
    // A failed reselect is reported; ordinary cleanup still restores the old one.
    let selected = unsafe { api.reselect_after_read(resources.memory_dc, resources.bitmap) };
    handle(selected, "SelectObject.after-GetDIBits")?;
    // SAFETY: Every u32 is initialized and all bit patterns are valid. This
    // immutable byte view covers exactly the checked, independently owned span
    // after native writes finish; it expires before the CPU owner is dropped.
    let bytes = unsafe { std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), length) };
    let image = converter
        .bgra_to_bgr(BgraView {
            pixels: bytes,
            width,
            height,
            stride,
        })
        .map_err(CaptureError::Conversion)?;
    Ok(WindowImage {
        image,
        width,
        height,
    })
}

fn handle<T, E>(value: *mut T, operation: &'static str) -> Result<(), CaptureError<E>> {
    if value.is_null() || value.addr() == usize::MAX {
        Err(CaptureError::Native(NativeFailure::last(operation)))
    } else {
        Ok(())
    }
}

fn layout<E>(width: i32, height: i32) -> Result<(usize, usize), CaptureError<E>> {
    if width <= 0 || height <= 0 {
        return Err(CaptureError::InvalidDimensions);
    }
    let stride = usize::try_from(width)
        .map_err(|_| CaptureError::SizeOverflow)?
        .checked_mul(4)
        .ok_or(CaptureError::SizeOverflow)?;
    let length = stride
        .checked_mul(usize::try_from(height).map_err(|_| CaptureError::SizeOverflow)?)
        .ok_or(CaptureError::SizeOverflow)?;
    isize::try_from(length).map_err(|_| CaptureError::SizeOverflow)?;
    Ok((stride, length))
}

fn intersection<E>(
    canvas: (i32, i32),
    popup: (i32, i32),
    offset: (i64, i64),
) -> Result<Option<Overlay>, CaptureError<E>> {
    let right = offset
        .0
        .checked_add(i64::from(popup.0))
        .ok_or(CaptureError::SizeOverflow)?;
    let bottom = offset
        .1
        .checked_add(i64::from(popup.1))
        .ok_or(CaptureError::SizeOverflow)?;
    // The source's origin subtraction must fit int, but OpenCV's intersection
    // deliberately avoids overflowing rectangle endpoints. A representable
    // offset with an endpoint beyond int still has defined empty/clipped output.
    for value in [offset.0, offset.1] {
        i32::try_from(value).map_err(|_| CaptureError::SizeOverflow)?;
    }
    let x = offset.0.max(0);
    let y = offset.1.max(0);
    let width = right.min(i64::from(canvas.0)) - x;
    let height = bottom.min(i64::from(canvas.1)) - y;
    if width <= 0 || height <= 0 {
        return Ok(None);
    }
    let narrow = |value| i32::try_from(value).map_err(|_| CaptureError::SizeOverflow);
    Ok(Some(Overlay {
        source_x: narrow(x - offset.0)?,
        source_y: narrow(y - offset.1)?,
        destination_x: narrow(x)?,
        destination_y: narrow(y)?,
        width: narrow(width)?,
        height: narrow(height)?,
    }))
}

#[cfg(test)]
#[path = "print_window_tests.rs"]
mod tests;
