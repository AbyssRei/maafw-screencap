// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::native::*;
use std::time::Duration;

pub(crate) type Window = *mut std::ffi::c_void;
pub(crate) type Monitor = *mut std::ffi::c_void;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MonitorRects {
    pub monitor: Rect,
    pub work: Rect,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Position {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Order {
    Top,
    Topmost,
    NotTopmost,
}

/// Opaque OS handles only. Methods validate/query via Win32; they borrow no
/// target memory, retain no pointers and do not change caller DPI/COM context.
/// All calls, source waits and window effects run synchronously on the caller.
pub(crate) trait WindowApi {
    fn valid(&self, window: Window) -> bool;
    fn foreground(&self) -> Window;
    fn foreground_request(&self, window: Window);
    fn order(&self, window: Window, order: Order, show: bool);
    fn ticks(&self) -> u32;
    fn sleep(&self, duration: Duration);
    fn zoomed(&self, window: Window) -> bool;
    fn iconic(&self, window: Window) -> bool;
    fn monitor(&self, window: Window) -> Monitor;
    fn monitor_rects(&self, monitor: Monitor) -> Result<MonitorRects, u32>;
    fn window_rect(&self, window: Window) -> Result<Rect, u32>;
    fn client_rect(&self, window: Window) -> Result<Rect, u32>;
    fn origin(&self, window: Window, point: Point) -> Result<Point, u32>;
    fn position(&self, window: Window, position: Position) -> Result<(), u32>;
    fn dpi(&self, window: Window) -> u32;
    fn style(&self, window: Window) -> isize;
}
pub(crate) struct SystemWindow;
fn rect(value: RECT) -> Rect {
    Rect {
        left: value.left,
        top: value.top,
        right: value.right,
        bottom: value.bottom,
    }
}
fn query_rect(
    window: Window,
    query: unsafe extern "system" fn(HWND, *mut RECT) -> i32,
) -> Result<Rect, u32> {
    let mut value = RECT::default();
    // SAFETY: Fixed Win32 query signature, opaque HWND, exact initialized output.
    unsafe {
        if query(window, &mut value) == 0 {
            Err(GetLastError())
        } else {
            Ok(rect(value))
        }
    }
}
impl WindowApi for SystemWindow {
    fn valid(&self, window: Window) -> bool {
        // SAFETY: OS validates a non-owning identifier without Rust dereference.
        unsafe { IsWindow(window) != 0 }
    }
    fn foreground(&self) -> Window {
        // SAFETY: No pointer inputs or ownership transfer.
        unsafe { GetForegroundWindow() }
    }
    fn foreground_request(&self, window: Window) {
        // SAFETY: Same original request; the final foreground query determines
        // its outcome. A rejected request is not treated as capture failure in U.
        let _ = unsafe { SetForegroundWindow(window) };
    }
    fn order(&self, window: Window, order: Order, show: bool) {
        let after = match order {
            Order::Top => HWND_TOP,
            Order::Topmost => HWND_TOPMOST,
            Order::NotTopmost => HWND_NOTOPMOST,
        };
        let flags = SWP_NOMOVE | SWP_NOSIZE | if show { SWP_SHOWWINDOW } else { 0 };
        // SAFETY: Fixed z-order operation; size/position arguments are unused.
        let _ = unsafe { SetWindowPos(window, after, 0, 0, 0, 0, flags as u32) };
    }
    fn ticks(&self) -> u32 {
        // SAFETY: DWORD clock only; wraparound is handled explicitly by caller.
        unsafe { GetTickCount() }
    }
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
    fn zoomed(&self, window: Window) -> bool {
        // SAFETY: Non-owning query, no retained memory.
        unsafe { IsZoomed(window) != 0 }
    }
    fn iconic(&self, window: Window) -> bool {
        // SAFETY: Non-owning query, no retained memory.
        unsafe { IsIconic(window) != 0 }
    }
    fn monitor(&self, window: Window) -> Monitor {
        // SAFETY: Same original nearest-monitor selection, no acquired lease.
        unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST as u32) }
    }
    fn monitor_rects(&self, monitor: Monitor) -> Result<MonitorRects, u32> {
        let mut info = MONITORINFO {
            cbSize: u32::try_from(size_of::<MONITORINFO>()).expect("fixed MONITORINFO size"),
            ..Default::default()
        };
        // SAFETY: Exact initialized structure/prefix for this non-owning query.
        unsafe {
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                Err(GetLastError())
            } else {
                Ok(MonitorRects {
                    monitor: rect(info.rcMonitor),
                    work: rect(info.rcWork),
                })
            }
        }
    }
    fn window_rect(&self, window: Window) -> Result<Rect, u32> {
        query_rect(window, GetWindowRect)
    }
    fn client_rect(&self, window: Window) -> Result<Rect, u32> {
        query_rect(window, GetClientRect)
    }
    fn origin(&self, window: Window, point: Point) -> Result<Point, u32> {
        let mut point = POINT {
            x: point.x,
            y: point.y,
        };
        // SAFETY: Initialized exact POINT; Win32 validates the opaque handle.
        unsafe {
            if ClientToScreen(window, &mut point) == 0 {
                Err(GetLastError())
            } else {
                Ok(Point {
                    x: point.x,
                    y: point.y,
                })
            }
        }
    }
    fn position(&self, window: Window, position: Position) -> Result<(), u32> {
        let flags = SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS;
        // SAFETY: Preserve original async window-position flags and caller DPI.
        // Integer inputs were checked; success is queueing, not a geometry wait.
        unsafe {
            if SetWindowPos(
                window,
                std::ptr::null_mut(),
                position.x,
                position.y,
                position.width,
                position.height,
                flags as u32,
            ) == 0
            {
                Err(GetLastError())
            } else {
                Ok(())
            }
        }
    }
    fn dpi(&self, window: Window) -> u32 {
        // SAFETY: Same non-compatible source API, no DPI-context mutation.
        unsafe { GetDpiForWindow(window) }
    }
    fn style(&self, window: Window) -> isize {
        // SAFETY: Fixed source style index, no borrowed pointers.
        unsafe { GetWindowLongPtrW(window, GWL_STYLE) }
    }
}

#[cfg(test)]
#[path = "window_api_tests.rs"]
mod tests;
