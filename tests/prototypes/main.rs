// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#![feature(allocator_ext)]
#![deny(unsafe_op_in_unsafe_fn)]

mod allocation;

#[cfg(all(target_os = "windows", feature = "prototype-windows"))]
mod windows;

#[cfg(all(target_os = "windows", feature = "prototype-windows-com"))]
mod windows_com;
