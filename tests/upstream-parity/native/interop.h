// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <stddef.h>
#include <stdint.h>

// Private P0 verification boundary. This is not the product SDK ABI.
#ifdef _WIN32
#define MFS_P0_EXPORT __declspec(dllexport)
#else
#define MFS_P0_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MfsP0Span {
    const uint8_t* data;
    size_t length;
    uint32_t width;
    uint32_t height;
    uint32_t stride;
    uint32_t channels;
} MfsP0Span;

// The span and its pixels are borrowed only for the callback. The worker owns
// context access until stop succeeds. stop from that worker's callback returns
// 2 without consuming the handle. Other overlapping stop calls are prohibited.
typedef void (*MfsP0Callback)(void* context, void* worker, const MfsP0Span* span, int32_t status);
// fault: 0=normal, 1=constructor exception, 2=conversion exception (tests only).
MFS_P0_EXPORT int32_t mfs_p0_start(void* context, MfsP0Callback callback, uint32_t fault, void** output);
MFS_P0_EXPORT int32_t mfs_p0_stop(void* worker);
MFS_P0_EXPORT int32_t mfs_p0_layout(uint64_t* output, size_t capacity);
MFS_P0_EXPORT const char* mfs_p0_opencv_info(void);

#ifdef __cplusplus
}
#endif
