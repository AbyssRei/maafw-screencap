// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"

// Test-only boundary. A failure bit is 1 << (operation - 1). The special
// bitmap/short-read probes initialize the complete reference input first;
// they observe the original branch without reading uninitialized C++ pixels.
#define MFS_GDI_GET_DC 1u
#define MFS_GDI_CREATE_DC 2u
#define MFS_GDI_CLIENT_RECT 3u
#define MFS_GDI_CREATE_BITMAP 4u
#define MFS_GDI_SELECT 5u
#define MFS_GDI_COPY 6u
#define MFS_GDI_OBJECT 7u
#define MFS_GDI_READ 8u
#define MFS_GDI_CONVERT 9u
#define MFS_GDI_RESTORE 10u
#define MFS_GDI_DELETE_BITMAP 11u
#define MFS_GDI_DELETE_DC 12u
#define MFS_GDI_RELEASE_DC 13u
#define MFS_GDI_BITMAP_24 (1u << 29)
#define MFS_GDI_SHORT_READ (1u << 30)
#define MFS_GDI_PROBE_EVENTS 32u

typedef struct MfsGdiEvent {
    uint32_t operation;
    int32_t result;
    uint32_t last_error;
} MfsGdiEvent;

typedef struct MfsGdiReport {
    uint32_t count;
    uint32_t overflow;
    uint32_t before;
    uint32_t after_capture;
    uint32_t after_fixture;
    uint32_t fixture_errors;
    uint32_t bitmap_depth;
    MfsGdiEvent events[MFS_GDI_PROBE_EVENTS];
} MfsGdiReport;

typedef struct MfsGdiApi {
    void* (*get_dc)(void* hwnd);
    void* (*create_dc)(void* dc);
    int32_t (*client_rect)(void* hwnd, void* rectangle);
    void* (*create_bitmap)(void* dc, int32_t width, int32_t height);
    void* (*select)(void* dc, void* object);
    int32_t (*copy)(void* destination, int32_t width, int32_t height, void* source);
    int32_t (*object)(void* bitmap, int32_t length, void* output);
    int32_t (*read)(void* bitmap, int32_t length, void* output);
    int32_t (*delete_bitmap)(void* bitmap);
    int32_t (*delete_dc)(void* dc);
    int32_t (*release_dc)(void* hwnd, void* dc);
} MfsGdiApi;

#ifdef __cplusplus
extern "C" {
#endif
// begin/end are caller-thread-affine and may not overlap/nest. end records
// native resources left by capture, then separately reclaims fixture-owned
// residuals after the original/Rust cleanup attempts. It never hides a leak.
MFS_PARITY_EXPORT int32_t mfs_gdi_probe_begin(uint32_t failures);
MFS_PARITY_EXPORT int32_t mfs_gdi_probe_end(MfsGdiReport* output);
MFS_PARITY_EXPORT int32_t mfs_gdi_probe_api(MfsGdiApi* output);
MFS_PARITY_EXPORT int32_t mfs_gdi_probe_reference(void* hwnd, uint32_t failures,
    MfsParityImage* image, MfsGdiReport* report);
MFS_PARITY_EXPORT int32_t mfs_gdi_probe_layout(uint64_t* output, size_t capacity);
MFS_PARITY_EXPORT uint64_t mfs_gdi_parity_image_owners(void);
#ifdef __cplusplus
}
#endif
