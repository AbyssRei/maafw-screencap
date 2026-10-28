// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::windows::gdi_api::DcApi;
use std::{cell::Cell, ffi::c_void};

#[derive(Default)]
struct Witness {
    dc: Cell<HDC>,
    bitmap: Cell<HBITMAP>,
    selected: Cell<HGDIOBJ>,
    reads: Cell<usize>,
    selections: Cell<usize>,
}
// SAFETY: All operations forward to SystemGdi on this same test thread. The
// witness observes real objects, retaining/writing no additional native memory.
unsafe impl DcApi for Witness {
    unsafe fn get_dc(&self, hwnd: HWND) -> HDC {
        unsafe { SystemGdi.get_dc(hwnd) }
    }
    unsafe fn create_dc(&self, dc: HDC) -> HDC {
        let result = unsafe { SystemGdi.create_dc(dc) };
        self.dc.set(result);
        result
    }
    unsafe fn client_rect(&self, hwnd: HWND, rectangle: *mut RECT) -> i32 {
        unsafe { SystemGdi.client_rect(hwnd, rectangle) }
    }
    unsafe fn create_bitmap(&self, dc: HDC, width: i32, height: i32) -> HBITMAP {
        let result = unsafe { SystemGdi.create_bitmap(dc, width, height) };
        self.bitmap.set(result);
        result
    }
    unsafe fn select(&self, dc: HDC, object: HGDIOBJ) -> HGDIOBJ {
        let result = unsafe { SystemGdi.select(dc, object) };
        if !result.is_null() && result.addr() != usize::MAX {
            self.selected.set(object);
            self.selections
                .set(self.selections.get().checked_add(1).unwrap());
        }
        result
    }
    unsafe fn delete_bitmap(&self, bitmap: HBITMAP) -> i32 {
        let result = unsafe { SystemGdi.delete_bitmap(bitmap) };
        if result != 0 {
            self.bitmap.set(std::ptr::null_mut());
        }
        result
    }
    unsafe fn delete_dc(&self, dc: HDC) -> i32 {
        let result = unsafe { SystemGdi.delete_dc(dc) };
        if result != 0 {
            self.dc.set(std::ptr::null_mut());
            self.selected.set(std::ptr::null_mut());
        }
        result
    }
    unsafe fn release_dc(&self, hwnd: HWND, dc: HDC) -> i32 {
        unsafe { SystemGdi.release_dc(hwnd, dc) }
    }
}
// SAFETY: Exact same-thread forwarding; the read query verifies Win32's
// unselected-bitmap precondition before forwarding the checked writable span.
unsafe impl PrintApi for Witness {
    unsafe fn is_window(&self, hwnd: HWND) -> i32 {
        unsafe { SystemGdi.is_window(hwnd) }
    }
    unsafe fn root_owner(&self, hwnd: HWND) -> HWND {
        unsafe { SystemGdi.root_owner(hwnd) }
    }
    unsafe fn last_popup(&self, hwnd: HWND) -> HWND {
        unsafe { SystemGdi.last_popup(hwnd) }
    }
    unsafe fn visible(&self, hwnd: HWND) -> i32 {
        unsafe { SystemGdi.visible(hwnd) }
    }
    unsafe fn origin(&self, hwnd: HWND, point: *mut POINT) -> i32 {
        unsafe { SystemGdi.origin(hwnd, point) }
    }
    unsafe fn print(&self, hwnd: HWND, dc: HDC) -> i32 {
        unsafe { SystemGdi.print(hwnd, dc) }
    }
    unsafe fn dibits(
        &self,
        dc: HDC,
        bitmap: HBITMAP,
        height: u32,
        pixels: *mut c_void,
        info: *mut BITMAPINFO,
    ) -> i32 {
        assert_eq!(self.dc.get(), dc);
        assert_eq!(
            pixels.addr() % align_of::<u32>(),
            0,
            "DIB rows have native DWORD alignment"
        );
        // SAFETY: The capture owns this live DC; query does not transfer ownership.
        let selected = unsafe { GetCurrentObject(dc, u32::try_from(OBJ_BITMAP).unwrap()) };
        assert_ne!(
            selected, bitmap,
            "GetDIBits bitmap must actually be unselected"
        );
        assert_eq!(selected, self.selected.get());
        // SAFETY: Capture supplied this valid initialized BITMAPINFO for the call.
        let header = unsafe { &(*info).bmiHeader };
        assert_eq!(header.biHeight, -i32::try_from(height).unwrap());
        assert_eq!(
            (header.biPlanes, header.biBitCount, header.biCompression),
            (1, 32, 0)
        );
        self.reads.set(self.reads.get().checked_add(1).unwrap());
        unsafe { SystemGdi.dibits(dc, bitmap, height, pixels, info) }
    }
}

struct ConversionWitness<'a> {
    native: &'a Witness,
    adapter: &'a Adapter,
}
impl BgraConverter for ConversionWitness<'_> {
    type Image = Image;
    type Error = i32;
    fn bgra_to_bgr(&mut self, input: BgraView<'_>) -> Result<Image, i32> {
        let dc = self.native.dc.get();
        assert!(!dc.is_null());
        // SAFETY: This is the still-owned memory DC, on the capture caller thread.
        assert_eq!(
            unsafe { GetCurrentObject(dc, u32::try_from(OBJ_BITMAP).unwrap()) },
            self.native.bitmap.get(),
            "original selected-bitmap state is restored before conversion"
        );
        self.adapter.converter(0).bgra_to_bgr(input)
    }
}
impl PrintConverter for ConversionWitness<'_> {
    fn overlay(
        &mut self,
        destination: &mut Image,
        source: &Image,
        rectangle: Overlay,
    ) -> Result<(), i32> {
        assert!(
            self.native.dc.get().is_null() && self.native.bitmap.get().is_null(),
            "native cleanup finishes before original BGR overlay"
        );
        self.adapter
            .converter(0)
            .overlay(destination, source, rectangle)
    }
}

#[test]
#[ignore = "explicit real Win32 selected-object read-boundary proof"]
fn unselects_only_during_dib_read_and_restores_before_conversion() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let mut fixture = adapter.print_fixture((25, 17), (5, 3), 1)?;
    let baseline = adapter.objects();
    let witness = Witness::default();
    let mut converter = ConversionWitness {
        native: &witness,
        adapter: &adapter,
    };
    let actual = PrintWindowCapture::new(fixture.hwnd()).capture_with_api(&witness, &mut converter);
    assert!(actual.main_cleanup.0.iter().all(Option::is_none));
    assert!(actual.popup_cleanup.0.iter().all(Option::is_none));
    let image = actual.image.map_err(|e| format!("capture: {e:?}"))?;
    assert_eq!(witness.reads.get(), 2);
    assert_eq!(
        witness.selections.get(),
        8,
        "initial, detach, reselect and final restore per capture"
    );
    assert_eq!(adapter.objects(), baseline);
    assert_pixels(&image, (5, 3), (25, 17), 1);
    fixture.close()?;
    Ok(())
}
