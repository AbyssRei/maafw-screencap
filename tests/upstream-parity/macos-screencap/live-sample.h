// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private acceptance ABI; no SDK capability or capture API.
#pragma once
#include <stdint.h>
#ifdef __cplusplus
#define MFS_LIVE_NOEXCEPT noexcept
extern "C" {
#else
#define MFS_LIVE_NOEXCEPT
#endif
// The sample is borrowed only during this callback, on the actual native
// completion thread, after the unchanged original completion has returned.
// Both callbacks must contain all exceptions/panics. No native sample escapes.
typedef int32_t (*MfsMacLiveVisit)(void*, const void*, uint32_t) MFS_LIVE_NOEXCEPT;
typedef void (*MfsMacLiveRelease)(void*) MFS_LIVE_NOEXCEPT;
// Query the actual compiled OpenCV tag; its integer encoding is not assumed.
__attribute__((visibility("default"))) uint64_t mfs_mac_original_live_bgr_type(void) MFS_LIVE_NOEXCEPT;
// A valid release callback transfers one context owner even on rejection.
// The original context and escaping observation blocks share that owner;
// release runs once after their final reference, on its actual ending thread.
// Null release is rejected without consuming context. Register once, before
// any of the original context's three calls. Retain the module until exit.
__attribute__((visibility("default"))) int32_t mfs_mac_original_set_live_sample(
    void*, void*, MfsMacLiveVisit, MfsMacLiveRelease) MFS_LIVE_NOEXCEPT;
#ifdef __cplusplus
}
#endif
