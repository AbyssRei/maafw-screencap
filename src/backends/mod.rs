// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#[cfg(all(
    target_os = "windows",
    any(
        feature = "windows-gdi",
        feature = "windows-print-window",
        feature = "windows-screen-dc",
        feature = "windows-frame-pool"
    )
))]
mod windows;
