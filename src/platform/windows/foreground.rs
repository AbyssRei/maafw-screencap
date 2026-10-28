// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from fixed MaaFramework Base/ForegroundUtils.h.
use super::window_api::{Order, SystemWindow, Window, WindowApi};
use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

// Same module-wide, cross-window cooldown as the original inline static DWORD.
// Atomics adapt its data-race-prone shared storage; they do not serialize native
// requests/sleeps or introduce the P3 window coordinator/lease policy.
static LAST_ATTEMPT: AtomicU32 = AtomicU32::new(0);

pub(crate) fn ensure_foreground(window: Window) -> bool {
    with_cooldown(&SystemWindow, window, &LAST_ATTEMPT)
}
pub(crate) fn with_cooldown<A: WindowApi>(api: &A, window: Window, last: &AtomicU32) -> bool {
    if window.is_null() || !api.valid(window) {
        return false;
    }
    if window == api.foreground() {
        return true;
    }
    let now = api.ticks();
    let previous = last.load(Ordering::SeqCst);
    // DWORD subtraction deliberately wraps at 2^32, including source's zero
    // sentinel behavior; no elapsed-time conversion to a different clock.
    if previous != 0 && now.wrapping_sub(previous) < 5000 {
        return window == api.foreground();
    }
    last.store(now, Ordering::SeqCst);
    ensure_topmost(api, window);
    window == api.foreground()
}
pub(crate) fn ensure_topmost<A: WindowApi>(api: &A, window: Window) {
    if window.is_null() {
        return;
    }
    if window != api.foreground() {
        api.order(window, Order::Top, true);
        api.sleep(Duration::from_millis(5));
        api.foreground_request(window);
        api.sleep(Duration::from_millis(10));
        if window != api.foreground() {
            api.order(window, Order::Topmost, false);
            api.order(window, Order::NotTopmost, false);
            api.sleep(Duration::from_millis(5));
        }
    }
}

pub(crate) fn with_system_clock<A: WindowApi>(api: &A, window: Window) -> bool {
    with_cooldown(api, window, &LAST_ATTEMPT)
}
