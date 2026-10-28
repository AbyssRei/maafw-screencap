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
