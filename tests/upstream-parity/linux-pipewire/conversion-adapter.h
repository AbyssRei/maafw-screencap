// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private P1 acceptance ABI only. Not a public SDK header or release asset.
#pragma once
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
#define MFS_PW_NOEXCEPT noexcept
extern "C" {
#else
#define MFS_PW_NOEXCEPT
#endif
#define MFS_PW_EXPORT __attribute__((visibility("default")))
struct MfsPwBgrView {
    const uint8_t* pixels;
    size_t length;
    size_t stride;
    int32_t width;
    int32_t height;
};
// Modes: 1 BGRA2BGR, 2 RGBA2BGR, 3 RGB2BGR, 4 YUV2BGR_YUY2.
// Return 0 success, 1 invalid arguments/layout, 2 OpenCV exception,
// 3 allocation failure, 4 another contained exception. Failure clears output.
// Borrowed input is immutable and retained only for the synchronous call.
MFS_PW_EXPORT int32_t mfs_pw_convert(const uint8_t*,size_t,int32_t,int32_t,size_t,int32_t,int32_t,void**) MFS_PW_NOEXCEPT;
MFS_PW_EXPORT int32_t mfs_pw_bgr_view(const void*,struct MfsPwBgrView*) MFS_PW_NOEXCEPT;
MFS_PW_EXPORT void mfs_pw_bgr_release(void*) MFS_PW_NOEXCEPT;
MFS_PW_EXPORT uint64_t mfs_pw_bgr_owners(void) MFS_PW_NOEXCEPT;
// 0 ABI revision; 1 view size; 2 alignment; 3..7 field offsets in declared order.
MFS_PW_EXPORT size_t mfs_pw_adapter_layout(uint32_t) MFS_PW_NOEXCEPT;
#ifdef __cplusplus
}
#endif
