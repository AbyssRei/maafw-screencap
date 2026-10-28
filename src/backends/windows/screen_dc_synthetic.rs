// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Deterministic capture/resource boundaries. These are scripted API tests,
// distinct from real Win32 capture and exact-original helper observation.
use super::*;
use crate::backends::windows::{
    gdi_api::{DcApi, GdiApi},
    native::*,
};
use crate::platform::windows::window_api::{Monitor, MonitorRects, Order, Position, Rect};
use std::{cell::RefCell, ffi::c_void, rc::Rc, time::Duration};
type Events = Rc<RefCell<Vec<(&'static str, [i64; 4])>>>;
fn handle(id: usize) -> *mut c_void {
    std::ptr::without_provenance_mut(id)
}
struct Windows {
    events: Events,
    fail: &'static str,
    client: Rect,
}
impl Windows {
    fn record(&self, name: &'static str) {
        self.events.borrow_mut().push((name, [0; 4]));
    }
}
impl WindowApi for Windows {
    fn valid(&self, _: Window) -> bool {
        self.record("IsWindow");
        false
    }
    fn foreground(&self) -> Window {
        panic!("invalid helper target short circuits")
    }
    fn foreground_request(&self, _: Window) {
        panic!("invalid helper target short circuits")
    }
    fn order(&self, _: Window, _: Order, _: bool) {
        panic!("invalid helper target short circuits")
    }
    fn ticks(&self) -> u32 {
        panic!("invalid helper target short circuits")
    }
    fn sleep(&self, _: Duration) {
        panic!("invalid helper target short circuits")
    }
    fn zoomed(&self, _: Window) -> bool {
        panic!("invalid helper target short circuits")
    }
    fn iconic(&self, _: Window) -> bool {
        panic!("invalid helper target short circuits")
    }
    fn monitor(&self, _: Window) -> Monitor {
        panic!("invalid helper target short circuits")
    }
    fn monitor_rects(&self, _: Monitor) -> Result<MonitorRects, u32> {
        panic!("invalid helper target short circuits")
    }
    fn window_rect(&self, _: Window) -> Result<Rect, u32> {
        self.record("GetWindowRect");
        if self.fail == "GetWindowRect" {
            Err(77)
        } else {
            Ok(Rect::default())
        }
    }
    fn client_rect(&self, _: Window) -> Result<Rect, u32> {
        self.record("GetClientRect");
        if self.fail == "GetClientRect" {
            Err(78)
        } else {
            Ok(self.client)
        }
    }
    fn origin(&self, _: Window, p: Point) -> Result<Point, u32> {
        self.events
            .borrow_mut()
            .push(("ClientToScreen", [i64::from(p.x), i64::from(p.y), 0, 0]));
        if self.fail == "ClientToScreen" {
            Err(79)
        } else {
            Ok(Point { x: -120, y: 37 })
        }
    }
    fn position(&self, _: Window, _: Position) -> Result<(), u32> {
        panic!("invalid helper target short circuits")
    }
    fn dpi(&self, _: Window) -> u32 {
        panic!("not in ScreenDC source path")
    }
    fn style(&self, _: Window) -> isize {
        panic!("not in ScreenDC source path")
    }
}
struct Native {
    events: Events,
    primary: &'static str,
    cleanup: bool,
}
impl Native {
    fn record(&self, name: &'static str, args: [i64; 4]) -> bool {
        self.events.borrow_mut().push((name, args));
        self.primary != name
            && !(self.cleanup
                && matches!(name, "restore" | "DeleteObject" | "DeleteDC" | "ReleaseDC"))
    }
    fn object(&self, name: &'static str, id: usize, args: [i64; 4]) -> *mut c_void {
        if self.record(name, args) {
            handle(id)
        } else {
            null_mut()
        }
    }
}
// SAFETY: Script-only opaque identifiers, never dereferenced. Writable pointers
// are borrowed for exactly their checked span and no callback/resource escapes.
unsafe impl DcApi for Native {
    unsafe fn get_dc(&self, w: HWND) -> HDC {
        self.object("GetDC", 1, [i64::try_from(w.addr()).unwrap(), 0, 0, 0])
    }
    unsafe fn create_dc(&self, _: HDC) -> HDC {
        self.object("CreateCompatibleDC", 2, [0; 4])
    }
    unsafe fn client_rect(&self, _: HWND, _: *mut RECT) -> i32 {
        panic!("ScreenDC queries through window boundary")
    }
    unsafe fn create_bitmap(&self, _: HDC, w: i32, h: i32) -> HBITMAP {
        self.object(
            "CreateCompatibleBitmap",
            3,
            [i64::from(w), i64::from(h), 0, 0],
        )
    }
    unsafe fn select(&self, _: HDC, o: HGDIOBJ) -> HGDIOBJ {
        self.object(
            if o == handle(3) {
                "SelectObject"
            } else {
                "restore"
            },
            4,
            [0; 4],
        )
    }
    unsafe fn delete_bitmap(&self, _: HBITMAP) -> i32 {
        i32::from(self.record("DeleteObject", [0; 4]))
    }
    unsafe fn delete_dc(&self, _: HDC) -> i32 {
        i32::from(self.record("DeleteDC", [0; 4]))
    }
    unsafe fn release_dc(&self, w: HWND, _: HDC) -> i32 {
        i32::from(self.record("ReleaseDC", [i64::try_from(w.addr()).unwrap(), 0, 0, 0]))
    }
}
// SAFETY: Script writes an exact BITMAP or exact initialized byte span only.
unsafe impl GdiApi for Native {
    unsafe fn copy(&self, _: HDC, _: i32, _: i32, _: HDC) -> i32 {
        panic!("must use screen-origin copy")
    }
    unsafe fn object(&self, _: HBITMAP, length: i32, output: *mut c_void) -> i32 {
        if !self.record("GetObjectW", [0; 4]) {
            return 0;
        }
        assert_eq!(usize::try_from(length).unwrap(), size_of::<BITMAP>());
        // SAFETY: Capture provides an aligned initialized BITMAP of this size.
        unsafe {
            output.cast::<BITMAP>().write(BITMAP {
                bmWidth: 2,
                bmHeight: 1,
                bmWidthBytes: 8,
                bmPlanes: 1,
                bmBitsPixel: 32,
                ..Default::default()
            });
        }
        length
    }
    unsafe fn read(&self, _: HBITMAP, length: i32, output: *mut c_void) -> i32 {
        if !self.record("GetBitmapBits", [0; 4]) {
            return 0;
        }
        assert_eq!(length, 8);
        // SAFETY: Capture provides eight initialized writable bytes here.
        unsafe {
            std::ptr::copy_nonoverlapping(
                [1_u8, 2, 3, 4, 5, 6, 7, 8].as_ptr(),
                output.cast::<u8>(),
                8,
            );
        }
        if self.primary == "short" { 7 } else { 8 }
    }
}
// SAFETY: Script observes arguments only and owns no actual OS objects.
unsafe impl ScreenDcApi for Native {
    unsafe fn copy_screen(&self, _: HDC, w: i32, h: i32, _: HDC, x: i32, y: i32) -> i32 {
        i32::from(self.record("BitBlt", [w, h, x, y].map(i64::from)))
    }
}
struct Converter {
    events: Events,
    fail: bool,
    panic: bool,
}
impl BgraConverter for Converter {
    type Image = [u8; 8];
    type Error = u32;
    fn bgra_to_bgr(&mut self, v: BgraView<'_>) -> Result<Self::Image, u32> {
        self.events.borrow_mut().push(("convert", [0; 4]));
        assert_eq!((v.width, v.height, v.stride), (2, 1, 8));
        assert!(!self.panic, "injected converter unwind");
        if self.fail {
            Err(99)
        } else {
            Ok(v.pixels.try_into().unwrap())
        }
    }
}
fn setup() -> (Events, Native, Windows, Converter) {
    let events = Events::default();
    (
        events.clone(),
        Native {
            events: events.clone(),
            primary: "",
            cleanup: false,
        },
        Windows {
            events: events.clone(),
            fail: "",
            client: Rect {
                left: 7,
                top: 9,
                right: 9,
                bottom: 10,
            },
        },
        Converter {
            events,
            fail: false,
            panic: false,
        },
    )
}
#[test]
fn preserves_nonfatal_coordination_screen_origin_null_dc_owner_and_conversion_order() {
    let (events, api, windows, mut converter) = setup();
    let attempt = ScreenDcCapture::new(handle(9)).capture_with_api(&api, &windows, &mut converter);
    let warnings = attempt.coordination.unwrap();
    assert!(!warnings.foreground);
    assert_eq!(warnings.geometry, Err(geometry::Error::InvalidWindow));
    assert_eq!(attempt.image.unwrap(), [1, 2, 3, 4, 5, 6, 7, 8]);
    assert!(attempt.cleanup.0.iter().all(Option::is_none));
    assert_eq!(
        events.borrow().iter().map(|e| e.0).collect::<Vec<_>>(),
        [
            "IsWindow",
            "IsWindow",
            "GetWindowRect",
            "GetClientRect",
            "ClientToScreen",
            "GetDC",
            "CreateCompatibleDC",
            "CreateCompatibleBitmap",
            "SelectObject",
            "BitBlt",
            "GetObjectW",
            "GetBitmapBits",
            "convert",
            "restore",
            "DeleteObject",
            "DeleteDC",
            "ReleaseDC"
        ]
    );
    for (operation, args) in events.borrow().iter() {
        match *operation {
            "ClientToScreen" => assert_eq!(*args, [7, 9, 0, 0]),
            "BitBlt" => assert_eq!(*args, [2, 1, -120, 37]),
            "GetDC" | "ReleaseDC" => assert_eq!(*args, [0; 4]),
            _ => (),
        }
    }
}
#[test]
fn rejects_null_query_failures_and_invalid_sizes_before_dc_acquisition() {
    let (events, api, mut windows, mut converter) = setup();
    let null = ScreenDcCapture::new(null_mut()).capture_with_api(&api, &windows, &mut converter);
    assert!(matches!(null.image, Err(CaptureError::NullWindow)));
    assert!(null.coordination.is_none());
    assert!(events.borrow().is_empty());
    for (op, code) in [
        ("GetWindowRect", 77),
        ("GetClientRect", 78),
        ("ClientToScreen", 79),
    ] {
        events.borrow_mut().clear();
        windows.fail = op;
        let a = ScreenDcCapture::new(handle(9)).capture_with_api(&api, &windows, &mut converter);
        assert!(
            matches!(a.image,Err(CaptureError::Native(NativeFailure{operation,last_error})) if operation==op && last_error==code)
        );
        assert!(!events.borrow().iter().any(|e| e.0 == "GetDC"));
    }
    windows.fail = "";
    for right in [7, 6, i32::MIN] {
        events.borrow_mut().clear();
        windows.client.right = right;
        let a = ScreenDcCapture::new(handle(9)).capture_with_api(&api, &windows, &mut converter);
        assert!(matches!(
            a.image,
            Err(CaptureError::InvalidDimensions | CaptureError::SizeOverflow)
        ));
        assert!(!events.borrow().iter().any(|e| e.0 == "GetDC"));
    }
}
#[test]
fn retains_primary_and_each_cleanup_failure_and_releases_once_on_unwind() {
    for op in [
        "GetDC",
        "CreateCompatibleDC",
        "CreateCompatibleBitmap",
        "SelectObject",
        "BitBlt",
        "GetObjectW",
        "GetBitmapBits",
        "short",
        "convert",
        "",
    ] {
        let (events, mut api, windows, mut converter) = setup();
        api.primary = op;
        api.cleanup = true;
        converter.fail = op == "convert";
        let a = ScreenDcCapture::new(handle(9)).capture_with_api(&api, &windows, &mut converter);
        match op {
            "" => assert!(a.image.is_ok()),
            "convert" => assert!(matches!(a.image, Err(CaptureError::Conversion(99)))),
            "short" => assert!(matches!(
                a.image,
                Err(CaptureError::IncompleteRead {
                    copied: 7,
                    required: 8
                })
            )),
            _ => assert!(
                matches!(a.image,Err(CaptureError::Native(NativeFailure{operation,..})) if operation==op)
            ),
        }
        let names = events.borrow().iter().map(|e| e.0).collect::<Vec<_>>();
        for (i, name) in ["restore", "DeleteObject", "DeleteDC", "ReleaseDC"]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                names.iter().filter(|v| **v == name).count(),
                usize::from(a.cleanup.0[i].is_some()),
                "{op}: {name}"
            );
        }
        let order = names
            .iter()
            .filter_map(|name| {
                ["restore", "DeleteObject", "DeleteDC", "ReleaseDC"]
                    .iter()
                    .position(|c| c == name)
            })
            .collect::<Vec<_>>();
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
    }
    let (events, api, windows, mut converter) = setup();
    converter.panic = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ScreenDcCapture::new(
            handle(9)
        )
        .capture_with_api(&api, &windows, &mut converter)))
        .is_err()
    );
    assert_eq!(
        events
            .borrow()
            .iter()
            .rev()
            .take(4)
            .map(|e| e.0)
            .collect::<Vec<_>>(),
        ["ReleaseDC", "DeleteDC", "DeleteObject", "restore"]
    );
}
