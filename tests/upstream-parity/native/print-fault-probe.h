// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-probe.h"

// Common GDI operations retain their values. COPY means PrintWindow and READ
// returns scanlines here. POPUP_ONLY selects the second capture; other faults
// select the first. Read-precondition selections are distinct from final restore.
typedef struct MfsPrintFaultApi {
    int32_t (*print)(void* hwnd, void* dc);
    int32_t (*read)(void* dc, void* bitmap, uint32_t height, void* output, void* info);
    void* (*deselect)(void* dc, void* previous);
    void* (*reselect)(void* dc, void* bitmap);
} MfsPrintFaultApi;

#ifdef __cplusplus
extern "C" {
#endif
MFS_PARITY_EXPORT int32_t mfs_print_fault_api(MfsPrintFaultApi* output);
MFS_PARITY_EXPORT int32_t mfs_print_fault_layout(uint64_t* output, size_t capacity);
// begin/end and residual recovery use the common caller-thread probe contract.
// The source short-return probe defines its full CPU input before observation;
// no uninitialized tail is exposed by its original nonzero-only read branch.
MFS_PARITY_EXPORT int32_t mfs_print_fault_reference(void* hwnd, uint32_t failures,
    MfsParityImage* image, MfsGdiReport* report);
#ifdef __cplusplus
}
#endif
