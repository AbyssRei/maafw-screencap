// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <stddef.h>
#include <stdint.h>

// Private acceptance boundary, excluded from the product SDK and default build.
#define MFS_PARITY_EXPORT __declspec(dllexport)
#ifdef __cplusplus
extern "C" {
#endif

typedef struct MfsParityImage {
    void* owner;
    const uint8_t* data;
    size_t length;
    int32_t width;
    int32_t height;
    size_t stride;
} MfsParityImage;

typedef struct MfsParityWindow {
    void* hwnd;
    int32_t width;
    int32_t height;
} MfsParityWindow;

// Images own a distinct CPU cv::Mat. Its borrowed span stays valid until release.
// Status 0=success, 1=invalid input/no reference sample, 2=contained exception,
// 3=allocation failure. All image/window outputs are zero on failure.
MFS_PARITY_EXPORT int32_t mfs_gdi_parity_reference(void* hwnd, MfsParityImage* output);
MFS_PARITY_EXPORT int32_t mfs_gdi_parity_convert(const uint8_t* data, size_t length,
    int32_t width, int32_t height, size_t stride, uint32_t fault, MfsParityImage* output);
MFS_PARITY_EXPORT void mfs_gdi_parity_release(void* owner);
// Independent cv::Mat::clone, including source FramePool's two clone points.
// Owner must originate in this loaded module. fault=1 contains a clone error.
MFS_PARITY_EXPORT int32_t mfs_gdi_parity_clone(void* owner, uint32_t fault, MfsParityImage* output);
// Private DesktopDupWindow boundary: clone the logical BGR ROI into packed,
// target-only storage after the original full-monitor conversion point.
MFS_PARITY_EXPORT int32_t mfs_desktop_dup_crop(void* owner, int32_t x, int32_t y,
    int32_t width, int32_t height, MfsParityImage* output);
MFS_PARITY_EXPORT int32_t mfs_gdi_parity_layout(uint64_t* output, size_t capacity);
MFS_PARITY_EXPORT int32_t mfs_gdi_parity_window(int32_t width, int32_t height, MfsParityWindow* output);
MFS_PARITY_EXPORT int32_t mfs_gdi_parity_paint(void* hwnd, uint32_t phase);
MFS_PARITY_EXPORT int32_t mfs_gdi_parity_destroy(void* hwnd);
MFS_PARITY_EXPORT int32_t mfs_gdi_parity_iconic(void* hwnd);
MFS_PARITY_EXPORT uint32_t mfs_gdi_parity_objects(void);

#ifdef __cplusplus
}
#endif
