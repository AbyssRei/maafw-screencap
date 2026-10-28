// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"

// Test-only query scenarios: normal, null root, null popup, popup == root,
// invalid popup, hidden popup, failed main origin, failed popup origin.
// No capture, conversion, cleanup or pixel operation is interposed here.
typedef struct MfsPrintQueryEvent {
    uint32_t operation;
    uint32_t popup;
    int32_t result;
    uint32_t last_error;
} MfsPrintQueryEvent;
typedef struct MfsPrintQueryReport {
    uint32_t count;
    uint32_t overflow;
    MfsPrintQueryEvent events[16];
} MfsPrintQueryReport;
typedef struct MfsPrintQueryApi {
    void* (*root)(void* hwnd);
    void* (*popup)(void* hwnd);
    int32_t (*valid)(void* hwnd);
    int32_t (*visible)(void* hwnd);
    int32_t (*origin)(void* hwnd, void* point);
} MfsPrintQueryApi;
#ifdef __cplusplus
extern "C" {
#endif
// Caller-thread-affine, without nesting. end observes queries only and owns no
// native objects. Retain the module from begin until end, including unwinding.
MFS_PARITY_EXPORT int32_t mfs_print_query_begin(void* main, uint32_t scenario);
MFS_PARITY_EXPORT int32_t mfs_print_query_end(MfsPrintQueryReport* output);
MFS_PARITY_EXPORT int32_t mfs_print_query_api(MfsPrintQueryApi* output);
MFS_PARITY_EXPORT int32_t mfs_print_query_layout(uint64_t* output, size_t capacity);
MFS_PARITY_EXPORT int32_t mfs_print_query_reference(void* main, uint32_t scenario,
    MfsParityImage* image, MfsPrintQueryReport* report);
#ifdef __cplusplus
}
#endif
