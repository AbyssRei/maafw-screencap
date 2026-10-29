// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
pub(crate) mod pixel_buffer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Error {
    pub operation: &'static str,
    pub code: i64,
}
impl Error {
    pub const fn new(operation: &'static str, code: i64) -> Self {
        Self { operation, code }
    }
    pub const fn allocation() -> Self {
        Self::new("fallible ScreenCaptureKit allocation", -12)
    }
}
