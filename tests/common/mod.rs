// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
pub(crate) mod native_module;
// The current-thread queue/WinRT host is consumed by FramePool tests, including
// the manager runtime entry (windows-manager enables windows-frame-pool).
#[cfg(all(
    target_os = "windows",
    feature = "prototype-windows-com",
    feature = "windows-frame-pool"
))]
pub(crate) mod windows_capture_host;
