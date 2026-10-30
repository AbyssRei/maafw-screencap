// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"
#include "gdi-probe.h"
#ifdef __cplusplus
extern "C" {
#endif
MFS_PARITY_EXPORT int32_t mfs_screen_parity_reference(void* window, MfsParityImage* output);
// Direct pinned HwndUtils helper, with actual Win32 calls on the calling thread.
// The host separately observes asynchronous completion on its own UI fixture.
MFS_PARITY_EXPORT int32_t mfs_screen_parity_ensure_geometry(void* window);
// Two outputs: native GdiFlush BOOL and DwmFlush HRESULT. Status 1 only denotes
// invalid output storage; native failure remains in the respective output.
MFS_PARITY_EXPORT int32_t mfs_screen_fixture_present(int32_t* output, size_t capacity);
// Read-only diagnostic, 16 u64 slots: valid/visible/iconic, foreground/target,
// point window/root/target root, cloaked HRESULT/value, origin x/y, client w/h,
// extended style and DWORD tick. Signed coordinates/HRESULT use i64 bit values.
MFS_PARITY_EXPORT int32_t mfs_screen_window_observation(void* window, uint64_t* output, size_t capacity);
// Same resource ledger/failure bits as GDI; window helper/query calls remain
// native and are outside this DC/bitmap ledger. Residual recovery is separate.
MFS_PARITY_EXPORT int32_t mfs_screen_probe_reference(void* window, uint32_t failures,
    MfsParityImage* output, MfsGdiReport* report);
MFS_PARITY_EXPORT int32_t mfs_screen_probe_copy(void* destination, int32_t width,
    int32_t height, void* source, int32_t x, int32_t y);
#ifdef __cplusplus
}
#endif
