// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private acceptance ABI only, excluded from SDK headers and release exports.
#pragma once
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
#define MFS_MAC_NOEXCEPT noexcept
extern "C" {
#else
#define MFS_MAC_NOEXCEPT
#endif
#define MFS_MAC_EXPORT __attribute__((visibility("default")))
struct MfsMacResources { uint64_t descriptors,threads,ports; };
MFS_MAC_EXPORT int32_t mfs_mac_resource_snapshot(struct MfsMacResources*) MFS_MAC_NOEXCEPT;
MFS_MAC_EXPORT size_t mfs_mac_resource_layout(uint32_t) MFS_MAC_NOEXCEPT;
#ifdef __cplusplus
}
#endif
