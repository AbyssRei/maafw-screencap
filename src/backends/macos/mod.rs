// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#[cfg(any(all(target_os = "macos", feature = "macos-manager"), test))]
pub(crate) mod manager;
#[cfg(all(target_os = "macos", feature = "macos-screen-capture-kit"))]
pub(crate) mod screen_capture_kit;
