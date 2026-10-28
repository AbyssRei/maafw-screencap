// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"

typedef struct MfsPrintFixture {
    void* owner;
    void* hwnd;
    void* popup;
    int32_t width;
    int32_t height;
    int32_t popup_width;
    int32_t popup_height;
} MfsPrintFixture;

#ifdef __cplusplus
extern "C" {
#endif
MFS_PARITY_EXPORT int32_t mfs_print_parity_reference(void* hwnd, MfsParityImage* output);
// Image owners originate in this module; destination is exclusively borrowed.
MFS_PARITY_EXPORT int32_t mfs_print_parity_overlay(void* destination, void* source,
    int32_t source_x, int32_t source_y, int32_t destination_x, int32_t destination_y,
    int32_t width, int32_t height);
// mode: 0 no popup, 1 visible last-active owned popup, 2 hidden owned popup.
// Even a failed create may publish a partial owner: it must be closed exactly
// once on the caller thread while retaining this module, including on failure.
MFS_PARITY_EXPORT int32_t mfs_print_parity_fixture(int32_t width, int32_t height,
    int32_t popup_width, int32_t popup_height, int32_t offset_x, int32_t offset_y,
    uint32_t mode, MfsPrintFixture* output);
// Failure leaves a partial owner alive. Do not free its context/unload code.
MFS_PARITY_EXPORT int32_t mfs_print_parity_close(void** owner);
MFS_PARITY_EXPORT int32_t mfs_print_parity_stats(void* owner, uint64_t* output, size_t capacity);
MFS_PARITY_EXPORT int32_t mfs_print_parity_layout(uint64_t* output, size_t capacity);
// 0: nonempty, 1: empty, 2: invalid dimensions/offset. Six int32 outputs are
// source x/y, destination x/y, width/height. This uses the source cv::Rect path.
MFS_PARITY_EXPORT int32_t mfs_print_parity_intersection(int32_t canvas_width, int32_t canvas_height,
    int32_t popup_width, int32_t popup_height, int64_t offset_x, int64_t offset_y,
    int32_t* output, size_t capacity);
// Diagnostic source interposition, distinct from pristine reference capture.
// mode 0 observes source reads; mode 1 unselects only around its first read.
// 36 u64 values: read count, overflow, mode, DC count, then two 16-value reads.
MFS_PARITY_EXPORT int32_t mfs_print_probe_reference(void* hwnd, uint32_t mode,
    MfsParityImage* image, uint64_t* observations, size_t capacity);
#ifdef __cplusplus
}
#endif
