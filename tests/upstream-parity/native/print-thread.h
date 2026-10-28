// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "print-parity.h"
#ifdef __cplusplus
extern "C" {
#endif
// Same window/rendering fixture, with a dedicated UI owner and real GetMessage
// loop. Creation, statistics, activation, destruction and class unregistration
// run on that owner. Capture remains on the calling test thread.
// Even failed creation may publish a partial opaque owner requiring close.
MFS_PARITY_EXPORT int32_t mfs_print_thread_fixture(int32_t width, int32_t height,
    int32_t popup_width, int32_t popup_height, int32_t offset_x, int32_t offset_y,
    uint32_t mode, MfsPrintFixture* output);
MFS_PARITY_EXPORT int32_t mfs_print_thread_child_fixture(int32_t width, int32_t height,
    int32_t popup_width, int32_t popup_height, int32_t offset_x, int32_t offset_y,
    uint32_t mode, MfsPrintFixture* output);
MFS_PARITY_EXPORT int32_t mfs_print_thread_stats(void* owner, uint64_t* output, size_t capacity);
// Success joins the UI thread before deleting/nulling the opaque owner. Failure
// retains context/worker/registration: keep the module loaded, do not retry.
MFS_PARITY_EXPORT int32_t mfs_print_thread_close(void** owner);
// UI owner id, calling id, actual HWND owner id, process id.
MFS_PARITY_EXPORT int32_t mfs_print_thread_info(void* owner, uint64_t* output, size_t capacity);
MFS_PARITY_EXPORT int32_t mfs_print_thread_activate(void* owner);
// Last explicit activation: SetForegroundWindow BOOL, active HWND, foreground
// HWND, target HWND, foreground process id. Observations do not bypass policy.
MFS_PARITY_EXPORT int32_t mfs_print_thread_activation_info(void* owner, uint64_t* output, size_t capacity);
// Creates an independent activation anchor on this same UI owner, then applies
// the existing minimize/layer setup there. SetActiveWindow must use that queue.
MFS_PARITY_EXPORT int32_t mfs_print_thread_prepare(void* owner, uint32_t layered);
#ifdef __cplusplus
}
#endif
