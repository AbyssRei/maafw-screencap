// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#[cfg(feature = "windows-desktop-dup")]
pub(crate) mod desktop_dup;
#[cfg(any(feature = "windows-screen-dc", feature = "windows-desktop-dup-window"))]
pub(crate) mod foreground;
#[cfg(any(feature = "windows-screen-dc", feature = "windows-desktop-dup-window"))]
pub(crate) mod geometry;
#[cfg(feature = "windows-manager")]
pub(crate) mod manager;
#[cfg(any(feature = "windows-print-window", feature = "windows-frame-pool"))]
pub(crate) mod pseudo_minimize;
#[cfg(all(test, feature = "prototype-windows"))]
pub(crate) mod test_library;
#[cfg(feature = "windows-frame-pool")]
pub(crate) mod wgc;
#[cfg(any(feature = "windows-screen-dc", feature = "windows-desktop-dup-window"))]
pub(crate) mod window_api;

#[cfg(any(
    feature = "windows-print-window",
    feature = "windows-screen-dc",
    feature = "windows-desktop-dup-window",
    feature = "windows-frame-pool"
))]
#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
mod native {
    include!(concat!(env!("OUT_DIR"), "/windows_window_coordination.rs"));
}
