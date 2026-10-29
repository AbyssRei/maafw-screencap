// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#[cfg(feature = "linux-pipewire")]
pub(crate) mod pipewire;
#[cfg(feature = "linux-wlr")]
pub(crate) mod wayland;
