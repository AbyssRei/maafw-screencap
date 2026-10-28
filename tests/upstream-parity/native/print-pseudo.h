// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"
#ifdef __cplusplus
extern "C" {
#endif
// Opaque original wrapper, serial creating-thread access. The module must stay
// loaded until close joins its real monitor; abandoning the owner must retain it.
MFS_PARITY_EXPORT int32_t mfs_print_pseudo_open(void* hwnd, void** owner);
MFS_PARITY_EXPORT int32_t mfs_print_pseudo_capture(void* owner, MfsParityImage* image);
MFS_PARITY_EXPORT int32_t mfs_print_pseudo_inactive(void* owner);
MFS_PARITY_EXPORT int32_t mfs_print_pseudo_close(void** owner);
// Fixture controls, separate from the original wrapper. State has six u64s:
// extended style, alpha (255 when not layered), iconic, foreground, client w/h.
MFS_PARITY_EXPORT int32_t mfs_print_pseudo_prepare(void* hwnd, void* anchor, uint32_t layered);
MFS_PARITY_EXPORT int32_t mfs_print_pseudo_state(void* hwnd, uint64_t* state, size_t capacity);
// Distinct opaque original PseudoMinimizeHelper owner. Its public source methods
// permit real-monitor observations without exposing the wrapper's private fields.
MFS_PARITY_EXPORT int32_t mfs_pseudo_helper_open(void* hwnd, void** owner);
MFS_PARITY_EXPORT int32_t mfs_pseudo_helper_ensure(void* owner);
MFS_PARITY_EXPORT int32_t mfs_pseudo_helper_status(void* owner, uint32_t* pseudo);
MFS_PARITY_EXPORT int32_t mfs_pseudo_helper_close(void** owner);
#ifdef __cplusplus
}
#endif
