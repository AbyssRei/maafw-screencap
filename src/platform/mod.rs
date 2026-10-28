// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#[cfg(all(
    target_os = "windows",
    any(
        feature = "windows-print-window",
        feature = "windows-screen-dc",
        feature = "windows-frame-pool",
        feature = "windows-desktop-dup"
    )
))]
pub(crate) mod windows;
