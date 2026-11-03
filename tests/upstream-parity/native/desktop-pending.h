// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"
#ifdef __cplusplus
extern "C" {
#endif
// Private acceptance owner, created/accessed/destroyed on the calling thread.
// Capture invokes the complete unchanged F class, then clones its BGR target
// only at this outward acceptance boundary to publish a packed CPU owner.
MFS_PARITY_EXPORT int32_t mfs_desktop_pending_create(void* hwnd, void** output);
MFS_PARITY_EXPORT int32_t mfs_desktop_pending_capture(void* owner, MfsParityImage* output);
MFS_PARITY_EXPORT int32_t mfs_desktop_pending_destroy(void** owner);
#ifdef __cplusplus
}
#endif
