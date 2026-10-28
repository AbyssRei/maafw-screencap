// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"
#ifdef __cplusplus
extern "C" {
#endif
MFS_PARITY_EXPORT int32_t mfs_screen_parity_reference(void* window, MfsParityImage* output);
// Two outputs: native GdiFlush BOOL and DwmFlush HRESULT. Status 1 only denotes
// invalid output storage; native failure remains in the respective output.
MFS_PARITY_EXPORT int32_t mfs_screen_fixture_present(int32_t* output, size_t capacity);
#ifdef __cplusplus
}
#endif
