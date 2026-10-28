// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Capture implementation under migration. No public SDK API is defined yet.
#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(
    any(
        all(target_os = "windows", feature = "windows-print-window"),
        all(
            test,
            target_os = "windows",
            any(feature = "windows-gdi", feature = "windows-screen-dc"),
            feature = "prototype-windows"
        )
    ),
    feature(allocator_ext)
)]

// P1 migrates individual backends before wiring the complete upstream manager.
// Keep them private until that path exists; do not expose internals for tests.
#[allow(dead_code)]
mod backends;
#[allow(dead_code)]
mod platform;

#[cfg(all(
    test,
    target_os = "windows",
    feature = "prototype-windows",
    any(
        feature = "windows-gdi",
        feature = "windows-print-window",
        feature = "windows-screen-dc"
    )
))]
#[path = "../tests/common/mod.rs"]
mod test_support;
