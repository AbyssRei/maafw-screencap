// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private native fixture ABI only; no SDK entry point.
#pragma once
#include <stdint.h>
#ifdef __cplusplus
#define MFS_SCENE_NOEXCEPT noexcept
extern "C" {
#else
#define MFS_SCENE_NOEXCEPT
#endif
// mode0: existing ScreenCaptureKit metadata scene; mode1: existing CG scene.
// Both strings are NUL-terminated UTF-8 and live for the synchronous call.
__attribute__((visibility("default"))) uint32_t mfs_mac_same_binary_scene_version(void) MFS_SCENE_NOEXCEPT;
__attribute__((visibility("default"))) int32_t mfs_mac_same_binary_scene_run(
    uint32_t mode, const char* executable, const char* directory) MFS_SCENE_NOEXCEPT;
#ifdef __cplusplus
}
#endif
