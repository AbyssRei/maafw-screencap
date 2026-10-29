// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Capture implementation under migration. No public SDK API is defined yet.
#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(
    all(
        test,
        target_os = "macos",
        feature = "macos-screen-capture-kit",
        feature = "prototype-macos"
    ),
    feature(test, custom_test_frameworks)
)]
#![cfg_attr(
    all(
        test,
        target_os = "macos",
        feature = "macos-screen-capture-kit",
        feature = "prototype-macos"
    ),
    test_runner(crate::macos_libtest_host::run)
)]
#![cfg_attr(
    any(
        test,
        all(target_os = "linux", feature = "linux-pipewire"),
        all(target_os = "linux", feature = "linux-wlr"),
        all(target_os = "macos", feature = "macos-screen-capture-kit"),
        all(
            target_os = "windows",
            any(feature = "windows-print-window", feature = "windows-frame-pool")
        ),
        all(
            test,
            target_os = "windows",
            any(
                feature = "windows-gdi",
                feature = "windows-screen-dc",
                feature = "windows-desktop-dup"
            ),
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

// Explicit acceptance host only. The original source fixture initializes AppKit
// on the process main thread and pumps it while synchronous capture callers run.
// Keep this outside the capture implementation and retain standard libtest's
// exact filtering, ignored handling, reporting and failure exit status.
#[cfg(all(
    test,
    target_os = "macos",
    feature = "macos-screen-capture-kit",
    feature = "prototype-macos"
))]
extern crate test;
#[cfg(all(
    test,
    target_os = "macos",
    feature = "macos-screen-capture-kit",
    feature = "prototype-macos"
))]
#[path = "../tests/upstream-parity/macos-screencap/libtest-host.rs"]
mod macos_libtest_host;

#[cfg(all(
    test,
    target_os = "windows",
    feature = "prototype-windows",
    any(
        feature = "windows-gdi",
        feature = "windows-print-window",
        feature = "windows-screen-dc",
        feature = "windows-frame-pool",
        feature = "windows-desktop-dup"
    )
))]
#[path = "../tests/common/mod.rs"]
mod test_support;
