// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"

typedef int64_t (*MfsWindowInvoke)(void*, uint32_t, void*, int64_t*);
typedef struct MfsWindowApi { void* context; MfsWindowInvoke invoke; } MfsWindowApi;
typedef struct MfsWindowResult { int64_t first, second; double scale; } MfsWindowResult;
#ifdef __cplusplus
extern "C" {
#endif
// Synchronous scripted calls only: no real desktop/scheduling evidence. The
// original cooldown static persists across calls while this DLL stays loaded.
// Six writable int64 slots are borrowed only during invoke; nothing is retained.
MFS_PARITY_EXPORT int32_t mfs_window_script(const MfsWindowApi*, uint32_t mode, void* window, MfsWindowResult*);
MFS_PARITY_EXPORT int32_t mfs_window_script_layout(uint64_t*, size_t);
#ifdef __cplusplus
}
#endif
