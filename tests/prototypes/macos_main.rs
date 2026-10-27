// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#![feature(allocator_ext)]
#![deny(unsafe_op_in_unsafe_fn)]

// AppKit requires the process main thread, so this explicit prototype has no
// libtest worker-thread harness. It is never part of the default test run.
#[cfg(target_os = "macos")]
mod macos_screen_capture;

fn main() -> std::process::ExitCode {
    #[cfg(target_os = "macos")]
    {
        macos_screen_capture::run()
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("p0-macos requires a native macOS host");
        std::process::ExitCode::FAILURE
    }
}
