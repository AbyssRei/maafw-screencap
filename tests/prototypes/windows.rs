// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// P0 only: a controlled HWND exercises the upstream GetDC/compatible bitmap/
// BitBlt/GetBitmapBits path. It adds no public SDK or desktop-capture backend.
#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
mod native {
    include!(concat!(env!("OUT_DIR"), "/windows_prototype.rs"));
}

use native::*;
use std::{error::Error, ptr::null_mut};

struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        // SAFETY: The test creates this HWND on the current thread and destroys
        // it on that same thread after all DCs referring to it have been released.
        assert_ne!(unsafe { DestroyWindow(self.0) }, 0);
    }
}

struct CaptureResources {
    window: HWND,
    dc: HDC,
    memory_dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
}
impl Drop for CaptureResources {
    fn drop(&mut self) {
        // SAFETY: All handles were acquired on this thread. Restore the selected
        // bitmap before deleting it and release the window DC with its owner HWND.
        unsafe {
            if !self.previous.is_null() {
                assert!(!SelectObject(self.memory_dc, self.previous).is_null());
            }
            if !self.bitmap.is_null() {
                assert_ne!(DeleteObject(self.bitmap), 0);
            }
            if !self.memory_dc.is_null() {
                assert_ne!(DeleteDC(self.memory_dc), 0);
            }
            if !self.dc.is_null() {
                assert_ne!(ReleaseDC(self.window, self.dc), 0);
            }
        }
    }
}

fn require_handle<T>(handle: *mut T, label: &str) -> Result<*mut T, Box<dyn Error>> {
    if handle.is_null() {
        // SAFETY: GetLastError has no pointer or ownership requirements.
        return Err(format!("{label}: Win32 error {}", unsafe { GetLastError() }).into());
    }
    Ok(handle)
}

#[test]
#[ignore = "explicit native desktop test; run with --ignored --test-threads=1"]
fn captures_controlled_window_and_keeps_pixels_after_native_cleanup() -> Result<(), Box<dyn Error>>
{
    let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
    // SAFETY: NUL-terminated class is a built-in window class. All optional
    // handles/parameters are null, and the HWND remains owned on this thread.
    let window = Window(require_handle(
        unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                class.as_ptr(),
                WS_POPUP | u32::try_from(WS_VISIBLE)?,
                80,
                80,
                32,
                24,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        },
        "CreateWindowExW",
    )?);
    let mut resources = CaptureResources {
        window: window.0,
        dc: null_mut(),
        memory_dc: null_mut(),
        bitmap: null_mut(),
        previous: null_mut(),
    };
    // SAFETY: The HWND is live. Resources are retained in acquisition order and
    // released by the guard, on this thread, on every early return.
    resources.dc = require_handle(unsafe { GetDC(window.0) }, "GetDC")?;
    resources.memory_dc = require_handle(
        unsafe { CreateCompatibleDC(resources.dc) },
        "CreateCompatibleDC",
    )?;
    let mut rectangle = RECT::default();
    // SAFETY: The output points to a writable RECT and the HWND is live.
    if unsafe { GetClientRect(window.0, &mut rectangle) } == 0 {
        return Err("GetClientRect failed".into());
    }
    let width = rectangle
        .right
        .checked_sub(rectangle.left)
        .ok_or("width overflow")?;
    let height = rectangle
        .bottom
        .checked_sub(rectangle.top)
        .ok_or("height overflow")?;
    if width <= 0 || height <= 0 {
        return Err("empty client area".into());
    }
    let length = usize::try_from(width)?
        .checked_mul(usize::try_from(height)?)
        .and_then(|v| v.checked_mul(4))
        .ok_or("buffer size overflow")?;
    let native_length = i32::try_from(length)?;
    // SAFETY: Matching DC, positive dimensions, checked output length.
    resources.bitmap = require_handle(
        unsafe { CreateCompatibleBitmap(resources.dc, width, height) },
        "CreateCompatibleBitmap",
    )?;
    // SAFETY: Both handles are live and belong to this thread; the old selection
    // is kept until restoration before bitmap destruction.
    resources.previous = require_handle(
        unsafe { SelectObject(resources.memory_dc, resources.bitmap) },
        "SelectObject",
    )?;
    let color = 0x00332211; // COLORREF: red=0x11, green=0x22, blue=0x33.
    // SAFETY: CreateSolidBrush has no pointer inputs.
    let brush = require_handle(unsafe { CreateSolidBrush(color) }, "CreateSolidBrush")?;
    // SAFETY: RECT and both GDI handles are valid. FillRect does not retain them.
    let fill = unsafe { FillRect(resources.dc, &rectangle, brush) };
    // SAFETY: FillRect completed and no DC has selected this brush.
    assert_ne!(unsafe { DeleteObject(brush) }, 0);
    if fill == 0 {
        return Err("FillRect failed".into());
    }
    // SAFETY: Live source/destination DCs, matching positive bitmap bounds.
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
        return Err("BitBlt failed".into());
    }
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(length)?;
    pixels.resize(length, 0_u8);
    // SAFETY: Bitmap is live, destination has exactly the checked writable span.
    let copied =
        unsafe { GetBitmapBits(resources.bitmap, native_length, pixels.as_mut_ptr().cast()) };
    if copied != native_length {
        return Err(format!("GetBitmapBits copied {copied}/{native_length}").into());
    }
    drop(resources);
    drop(window);
    assert_eq!(pixels.len(), length);
    for pixel in pixels.as_chunks::<4>().0 {
        assert_eq!(&pixel[..3], &[0x33, 0x22, 0x11]);
    }
    // This confirms independent CPU storage, including cross-thread release.
    std::thread::spawn(move || assert_eq!(&pixels[..3], &[0x33, 0x22, 0x11]))
        .join()
        .unwrap();
    Ok(())
}
