// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#[cfg(any(feature = "linux-pipewire", test))]
pub(crate) mod pipewire;
#[cfg(all(target_os = "linux", feature = "linux-wlr"))]
pub(crate) mod wlr;
