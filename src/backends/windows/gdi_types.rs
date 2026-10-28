// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::native::*;
use std::collections::TryReserveError;

pub(crate) use super::pixels::{BgraConverter, BgraView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeFailure {
    pub operation: &'static str,
    // Not every GDI API defines GetLastError. Preserve the observed value
    // without presenting it as a guaranteed API-specific error code.
    pub last_error: u32,
}

impl NativeFailure {
    pub(super) fn last(operation: &'static str) -> Self {
        Self {
            operation,
            // SAFETY: GetLastError has no pointer/ownership preconditions.
            last_error: unsafe { GetLastError() },
        }
    }
}

#[derive(Debug)]
pub(crate) enum CaptureError<E> {
    NullWindow,
    InvalidWindow,
    Native(NativeFailure),
    InvalidDimensions,
    SizeOverflow,
    UnsupportedBitmap,
    IncompleteRead { copied: i32, required: i32 },
    IncompleteScanlines { copied: i32, required: i32 },
    Allocation(TryReserveError),
    Conversion(E),
}

/// Independent evidence, rather than overwriting a primary capture error.
/// Slots follow the original restore/delete-bitmap/delete-DC/release-DC order.
#[derive(Debug, Default)]
pub(crate) struct Cleanup(pub [Option<NativeFailure>; 4]);
