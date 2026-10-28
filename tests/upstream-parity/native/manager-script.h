// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"

typedef struct MfsManagerStep {
    uint32_t outcome; // 0=nullopt, 1=pixels, 2=engaged empty Mat, 3=exception
    uint32_t reserved;
    uint64_t elapsed_ns;
    uint64_t image_drop_ns;
} MfsManagerStep;
typedef struct MfsManagerScript {
    uint64_t flags;
    uint32_t valid;
    uint32_t dpi_success;
    uint32_t fail_create;
    uint32_t reconnect;
    uint32_t null_window;
    uint32_t reserved;
    MfsManagerStep steps[6][4];
} MfsManagerScript;
typedef struct MfsManagerEvent {
    uint32_t kind; // create=1, capture=2, image create=3, image drop=4,
                   // unit drop=5, now=6, DPI=7, IsWindow=8, inactive=9
    uint32_t method;
    uint32_t call;
    uint32_t reserved;
    uint64_t time_ns;
} MfsManagerEvent;
typedef struct MfsManagerReport {
    uint32_t connect_status; // false=0, true=1, escaped=2
    uint32_t selected;
    uint32_t connected_before;
    uint32_t connected_after;
    uint32_t capture_status[2];
    uint32_t output_tokens[2];
    uint32_t reconnect_status; // 3=not requested
    uint32_t selected_after_reconnect;
    uint32_t trace_count;
    uint32_t overflow;
    MfsManagerEvent events[256];
} MfsManagerReport;
typedef struct MfsManagerGdiReport {
    uint32_t connected;
    uint32_t captured;
    uint32_t selected;
    uint32_t calls_after_connect;
    uint32_t calls;
    uint32_t threads[3];
    uint32_t dpi_v2[3];
    uint32_t reserved;
} MfsManagerGdiReport;
#ifdef __cplusplus
extern "C" {
#endif
// Exact fixed manager screenshot bodies, backend/platform/clock scripts and
// input=None stubs. This is NOT native backend/desktop or input acceptance.
MFS_PARITY_EXPORT int32_t mfs_manager_script(const MfsManagerScript*, MfsManagerReport*);
MFS_PARITY_EXPORT int32_t mfs_manager_script_layout(uint64_t*, size_t);
// Same screenshot bodies, real source GDI, real caller DPI and input=None.
// Other capture methods are deliberately unavailable in this narrow reference.
MFS_PARITY_EXPORT int32_t mfs_manager_gdi_reference(void*, MfsParityImage*, MfsManagerGdiReport*);
#ifdef __cplusplus
}
#endif
