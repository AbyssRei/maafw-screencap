// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#[cfg(feature = "windows-screen-dc")]
pub(crate) mod foreground;
#[cfg(feature = "windows-screen-dc")]
pub(crate) mod geometry;
#[cfg(feature = "windows-print-window")]
pub(crate) mod pseudo_minimize;
#[cfg(feature = "windows-frame-pool")]
pub(crate) mod wgc;
#[cfg(feature = "windows-screen-dc")]
pub(crate) mod window_api;

#[cfg(any(feature = "windows-print-window", feature = "windows-screen-dc"))]
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
