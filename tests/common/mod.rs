// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
pub(crate) mod native_module;
#[cfg(all(target_os = "windows", feature = "prototype-windows-com"))]
pub(crate) mod windows_capture_host;
