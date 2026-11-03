// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
pub(crate) mod native {
    include!(concat!(env!("OUT_DIR"), "/windows_desktop_dup.rs"));
}

#[cfg(all(test, feature = "prototype-windows"))]
#[path = "desktop_dup_map_probe.rs"]
pub(crate) mod map_probe;

#[cfg(all(test, feature = "prototype-windows"))]
#[path = "desktop_dup_init_probe.rs"]
pub(crate) mod init_probe;

#[cfg(all(test, feature = "prototype-windows"))]
#[path = "desktop_dup_output_probe.rs"]
pub(crate) mod output_probe;
