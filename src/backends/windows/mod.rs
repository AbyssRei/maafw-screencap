// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
mod gdi;

#[cfg(all(test, feature = "prototype-windows"))]
#[path = "../../../tests/upstream-parity/gdi_adapter.rs"]
mod parity_adapter;

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
