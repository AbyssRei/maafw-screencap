// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#[cfg(feature = "windows-manager")]
mod delegate;
#[cfg(feature = "windows-desktop-dup")]
mod desktop_dup;
#[cfg(feature = "windows-desktop-dup-window")]
mod desktop_dup_window;
#[cfg(feature = "windows-frame-pool")]
mod frame_pool;
#[cfg(feature = "windows-frame-pool")]
mod frame_pool_pseudo;
#[cfg(feature = "windows-gdi")]
mod gdi;
#[cfg(any(
    feature = "windows-gdi",
    feature = "windows-print-window",
    feature = "windows-screen-dc"
))]
mod gdi_api;
#[cfg(any(feature = "windows-gdi", feature = "windows-screen-dc"))]
mod gdi_read;
#[cfg(any(
    feature = "windows-gdi",
    feature = "windows-print-window",
    feature = "windows-screen-dc"
))]
mod gdi_resources;
#[cfg(any(
    feature = "windows-gdi",
    feature = "windows-print-window",
    feature = "windows-screen-dc"
))]
mod gdi_types;
#[cfg(feature = "windows-manager")]
pub(crate) mod manager;
mod pixels;
#[cfg(feature = "windows-print-window")]
mod print_window;
#[cfg(feature = "windows-print-window")]
mod print_window_api;
#[cfg(feature = "windows-print-window")]
mod print_window_pseudo;
#[cfg(feature = "windows-screen-dc")]
mod screen_dc;

#[cfg(all(test, feature = "prototype-windows"))]
#[path = "../../../tests/upstream-parity/gdi_adapter.rs"]
mod parity_adapter;

#[cfg(any(
    feature = "windows-gdi",
    feature = "windows-print-window",
    feature = "windows-screen-dc"
))]
#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
mod native {
    include!(concat!(env!("OUT_DIR"), "/windows_gdi.rs"));
}
