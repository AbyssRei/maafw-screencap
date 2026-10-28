// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from fixed MaaFramework ScreenDCScreencap.{h,cpp}. See notices.
use super::{
    gdi_api::{ScreenDcApi, SystemGdi},
    gdi_read,
    gdi_resources::{Resources, require_handle},
    gdi_types::*,
};
use crate::platform::windows::{
    foreground, geometry,
    window_api::{Point, SystemWindow, Window, WindowApi},
};
use std::{marker::PhantomData, ptr::null_mut, rc::Rc};

/// Nonfatal upstream warnings survive independently of capture/cleanup errors.
pub(crate) struct Coordination {
    pub foreground: bool,
    pub geometry: Result<(), geometry::Error>,
}
pub(crate) struct Attempt<T, E> {
    pub image: Result<T, CaptureError<E>>,
    pub coordination: Option<Coordination>,
    pub cleanup: Cleanup,
}

/// Window client pixels as visible on the desktop, including occlusion. A null
/// HWND is invalid, never a desktop target. All native use/cleanup and source
/// sleeps execute on the caller; no mobility/ownership claim for this HWND.
pub(crate) struct ScreenDcCapture {
    window: Window,
    _thread: PhantomData<Rc<()>>,
}
impl ScreenDcCapture {
    pub fn new(window: Window) -> Self {
        Self {
            window,
            _thread: PhantomData,
        }
    }
    pub fn capture<C: BgraConverter>(&mut self, converter: &mut C) -> Attempt<C::Image, C::Error> {
        self.capture_with_api(&SystemGdi, &SystemWindow, converter)
    }
    fn capture_with_api<A: ScreenDcApi, W: WindowApi, C: BgraConverter>(
        &mut self,
        api: &A,
        window_api: &W,
        converter: &mut C,
    ) -> Attempt<C::Image, C::Error> {
        // The owner of GetDC(nullptr) is nullptr, distinct from the target.
        let mut resources = Resources::new(null_mut(), api);
        let mut coordination = None;
        let image = self.capture_inner(&mut resources, window_api, converter, &mut coordination);
        let cleanup = resources.release();
        Attempt {
            image,
            coordination,
            cleanup,
        }
    }
    fn capture_inner<A: ScreenDcApi, W: WindowApi, C: BgraConverter>(
        &self,
        resources: &mut Resources<'_, A>,
        window_api: &W,
        converter: &mut C,
        coordination: &mut Option<Coordination>,
    ) -> Result<C::Image, CaptureError<C::Error>> {
        if self.window.is_null() {
            return Err(CaptureError::NullWindow);
        }
        let foreground = foreground::with_system_clock(window_api, self.window);
        let geometry = geometry::ensure_on_screen(window_api, self.window);
        *coordination = Some(Coordination {
            foreground,
            geometry,
        });
        // Preserve the source query even though it does not consume this rect.
        window_api
            .window_rect(self.window)
            .map_err(|code| native("GetWindowRect", code))?;
        let client = window_api
            .client_rect(self.window)
            .map_err(|code| native("GetClientRect", code))?;
        let origin = window_api
            .origin(
                self.window,
                Point {
                    x: client.left,
                    y: client.top,
                },
            )
            .map_err(|code| native("ClientToScreen", code))?;
        let width = client
            .right
            .checked_sub(client.left)
            .ok_or(CaptureError::SizeOverflow)?;
        let height = client
            .bottom
            .checked_sub(client.top)
            .ok_or(CaptureError::SizeOverflow)?;
        if width <= 0 || height <= 0 {
            return Err(CaptureError::InvalidDimensions);
        }
        // SAFETY: Acquire the screen DC on this thread; retain it through copy
        // and conversion. Resources matches its null owner in ReleaseDC.
        resources.dc = unsafe { resources.api.get_dc(null_mut()) };
        require_handle(resources.dc, "GetDC")?;
        // SAFETY: Live screen DC retained by Resources.
        resources.memory_dc = unsafe { resources.api.create_dc(resources.dc) };
        require_handle(resources.memory_dc, "CreateCompatibleDC")?;
        // SAFETY: Positive dimensions and live compatible source DC.
        resources.bitmap = unsafe { resources.api.create_bitmap(resources.dc, width, height) };
        require_handle(resources.bitmap, "CreateCompatibleBitmap")?;
        // SAFETY: Live destination DC/bitmap, previous object retained to restore.
        resources.previous = unsafe { resources.api.select(resources.memory_dc, resources.bitmap) };
        require_handle(resources.previous, "SelectObject")?;
        // SAFETY: Destination bitmap covers the requested client size. Screen
        // origin is passed unchanged, including negative virtual coordinates.
        if unsafe {
            resources.api.copy_screen(
                resources.memory_dc,
                width,
                height,
                resources.dc,
                origin.x,
                origin.y,
            )
        } == 0
        {
            return Err(CaptureError::Native(NativeFailure::last("BitBlt")));
        }
        gdi_read::convert(resources, width, height, converter)
    }
}
fn native<E>(operation: &'static str, last_error: u32) -> CaptureError<E> {
    CaptureError::Native(NativeFailure {
        operation,
        last_error,
    })
}

#[cfg(test)]
#[path = "screen_dc_tests.rs"]
mod tests;
