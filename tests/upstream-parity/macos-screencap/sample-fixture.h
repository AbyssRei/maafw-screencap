// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private native sample/conversion fixture ABI, never an SDK/release header.
#pragma once
#include "acceptance-evidence.h"
#include "../linux-pipewire/conversion-adapter.h"
#ifdef __cplusplus
extern "C" {
#endif
struct MfsMacSampleInfo {
    uint64_t width,height,stride,span,data_size,format,owner_thread,source_thread,mutation_thread;
};
// Cases 0..3, cycles 0..19; native references stay on their creator channel.
// Return 0 success, 1 bad call, 2 native/invariant failure (error query preserves code),
// 3 allocation failure, 4 contained OpenCV exception, 5 another exception.
// Pointer/record outputs are cleared on failure. No exception crosses this ABI.
MFS_MAC_EXPORT int32_t mfs_mac_sample_create(uint32_t,uint32_t,void**) MFS_MAC_NOEXCEPT;
MFS_MAC_EXPORT int32_t mfs_mac_sample_info(const void*,struct MfsMacSampleInfo*) MFS_MAC_NOEXCEPT;
// Borrowed actual CMSampleBufferRef. Valid only through the live fixture and
// before mutate/destroy; never transfer a native owner through this pointer.
MFS_MAC_EXPORT const void* mfs_mac_sample_buffer(const void*) MFS_MAC_NOEXCEPT;
MFS_MAC_EXPORT int32_t mfs_mac_sample_save(const void*,const char*) MFS_MAC_NOEXCEPT;
MFS_MAC_EXPORT int32_t mfs_mac_sample_mutate(void*) MFS_MAC_NOEXCEPT;
MFS_MAC_EXPORT void mfs_mac_sample_destroy(void*) MFS_MAC_NOEXCEPT;
// Calls the unchanged source image_from_sample_buffer and moves its CPU Mat
// into an independent owner. It contains no CF/native sample reference.
MFS_MAC_EXPORT int32_t mfs_mac_sample_source(void*,void**) MFS_MAC_NOEXCEPT;
MFS_MAC_EXPORT int32_t mfs_mac_sample_source_view(const void*,struct MfsPwBgrView*) MFS_MAC_NOEXCEPT;
MFS_MAC_EXPORT void mfs_mac_sample_source_release(void*) MFS_MAC_NOEXCEPT;
MFS_MAC_EXPORT size_t mfs_mac_sample_layout(uint32_t) MFS_MAC_NOEXCEPT;
// 0/1 fixture created/closed, 2/3 CPU source image created/closed,
// 4 mutations, 5 original conversions, 6/7 fixture write locks/unlocks.
MFS_MAC_EXPORT uint64_t mfs_mac_sample_counter(uint32_t) MFS_MAC_NOEXCEPT;
// 0 primary native error, 1 fixture cleanup error; independent evidence.
MFS_MAC_EXPORT int32_t mfs_mac_sample_error(uint32_t) MFS_MAC_NOEXCEPT;
MFS_MAC_EXPORT uint64_t mfs_mac_sample_release_thread(uint32_t) MFS_MAC_NOEXCEPT;
MFS_MAC_EXPORT uint64_t mfs_mac_sample_thread(void) MFS_MAC_NOEXCEPT;
#ifdef __cplusplus
}
#endif
