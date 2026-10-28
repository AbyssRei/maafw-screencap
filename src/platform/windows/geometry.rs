// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from fixed unpatched HwndUtils.hpp. Keep rcWork in U; P2 changes it.
use super::window_api::{Point, Position, Window, WindowApi};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    InvalidWindow,
    Native(&'static str, u32),
    Overflow,
}
fn add(left: i32, right: i32) -> Result<i32, Error> {
    left.checked_add(right).ok_or(Error::Overflow)
}
fn sub(left: i32, right: i32) -> Result<i32, Error> {
    left.checked_sub(right).ok_or(Error::Overflow)
}

pub(crate) fn window_scale<A: WindowApi>(api: &A, window: Window) -> f64 {
    f64::from(api.dpi(window)) / 96.0
}
pub(crate) fn window_size<A: WindowApi>(api: &A, window: Window) -> Result<(i32, i32), Error> {
    let Ok(client) = api.client_rect(window) else {
        return Ok((0, 0));
    };
    Ok((
        sub(client.right, client.left)?,
        sub(client.bottom, client.top)?,
    ))
}
pub(crate) fn is_fullscreen<A: WindowApi>(api: &A, window: Window) -> bool {
    // GWL_STYLE is a 32-bit style mask even though GetWindowLongPtr returns
    // LONG_PTR. Do not sign-extend WS_POPUP into an extra high-word predicate.
    api.style(window) & 0x8000_0000 != 0
}
pub(crate) fn ensure_on_screen<A: WindowApi>(api: &A, window: Window) -> Result<(), Error> {
    if window.is_null() || !api.valid(window) {
        return Err(Error::InvalidWindow);
    }
    if api.zoomed(window) || api.iconic(window) {
        return Ok(());
    }
    let monitor = api.monitor(window);
    if monitor.is_null() {
        return Err(Error::Native("MonitorFromWindow", 0));
    }
    let monitor = api
        .monitor_rects(monitor)
        .map_err(|code| Error::Native("GetMonitorInfoW", code))?
        .work;
    let monitor_width = sub(monitor.right, monitor.left)?;
    let monitor_height = sub(monitor.bottom, monitor.top)?;
    let outer = api
        .window_rect(window)
        .map_err(|code| Error::Native("GetWindowRect", code))?;
    let client = api
        .client_rect(window)
        .map_err(|code| Error::Native("GetClientRect", code))?;
    let origin = api
        .origin(window, Point::default())
        .map_err(|code| Error::Native("ClientToScreen", code))?;
    let frame_left = sub(origin.x, outer.left)?;
    let frame_top = sub(origin.y, outer.top)?;
    let frame_right = sub(outer.right, add(origin.x, client.right)?)?;
    let frame_bottom = sub(outer.bottom, add(origin.y, client.bottom)?)?;
    let mut width = sub(client.right, client.left)?;
    let mut height = sub(client.bottom, client.top)?;
    let (mut x, mut y) = (origin.x, origin.y);
    let mut changed = false;
    if width > monitor_width {
        width = monitor_width;
        changed = true;
    }
    if height > monitor_height {
        height = monitor_height;
        changed = true;
    }
    // Preserve source priority: right/bottom, then left/top override them.
    if add(x, width)? > monitor.right {
        x = sub(monitor.right, width)?;
        changed = true;
    }
    if add(y, height)? > monitor.bottom {
        y = sub(monitor.bottom, height)?;
        changed = true;
    }
    if x < monitor.left {
        x = monitor.left;
        changed = true;
    }
    if y < monitor.top {
        y = monitor.top;
        changed = true;
    }
    if !changed {
        return Ok(());
    }
    let position = Position {
        x: sub(x, frame_left)?,
        y: sub(y, frame_top)?,
        width: add(add(width, frame_left)?, frame_right)?,
        height: add(add(height, frame_top)?, frame_bottom)?,
    };
    api.position(window, position)
        .map_err(|code| Error::Native("SetWindowPos", code))
}
