// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// The portable consumer algorithm is tested on the host; that is not Linux
// native capture acceptance. Native daemon/SHM work has a separate explicit probe.
#[cfg(any(
    all(
        target_os = "linux",
        any(feature = "linux-pipewire", feature = "linux-wlr")
    ),
    test
))]
mod linux;
#[cfg(any(all(target_os = "macos", feature = "macos-screen-capture-kit"), test))]
mod macos;
#[cfg(test)]
#[path = "../../tests/upstream-parity/manager-metadata/compare.rs"]
mod manager_metadata;
#[cfg(all(
    target_os = "windows",
    any(
        feature = "windows-gdi",
        feature = "windows-print-window",
        feature = "windows-screen-dc",
        feature = "windows-frame-pool",
        feature = "windows-desktop-dup"
    )
))]
mod windows;
