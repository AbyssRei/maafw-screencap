// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <stddef.h>
#include <stdint.h>
#ifdef MFS_FRAMEPOOL_INIT_EXPORTS
#define MFS_INIT_API __declspec(dllexport)
#else
#define MFS_INIT_API __declspec(dllimport)
#endif
#ifdef __cplusplus
extern "C" {
#define MFS_INIT_NOEXCEPT noexcept
#else
#define MFS_INIT_NOEXCEPT
#endif
// Private acceptance ABI. All status returns are signed native HRESULT values.
// This fixture creates only hidden caller-thread windows, never capture sessions.
MFS_INIT_API uint32_t mfs_framepool_init_version(void) MFS_INIT_NOEXCEPT;
MFS_INIT_API uint64_t mfs_framepool_init_clock(void) MFS_INIT_NOEXCEPT;
// Six scalar widths: pointer, size_t, u64, i32, u32, native HRESULT.
MFS_INIT_API int32_t mfs_framepool_init_widths(uint32_t* output, size_t count) MFS_INIT_NOEXCEPT;
MFS_INIT_API int32_t mfs_framepool_init_window_create(void** output) MFS_INIT_NOEXCEPT;
// Caller must own the actual window; successful destruction clears its slot.
MFS_INIT_API int32_t mfs_framepool_init_window_destroy(void** window) MFS_INIT_NOEXCEPT;
// Four scalars: caller TID, window TID, native IsWindow, IsWindowVisible.
// A destroyed handle is only an observed scalar, never dereferenced.
MFS_INIT_API int32_t mfs_framepool_init_window_info(void* window, uint64_t* output, size_t count) MFS_INIT_NOEXCEPT;
// Borrow one live ID3D11Device on its caller channel. Its private-data owner
// retains a native IUnknown sentinel, not the device. Each index is used once.
MFS_INIT_API int32_t mfs_framepool_init_attach(void* device, uint32_t index) MFS_INIT_NOEXCEPT;
// Six scalars: attached, final releases, attach TID, final TID, attach QPC,
// final QPC. Native final Release may run on another thread; process-lifetime
// atomic records hold no borrowed caller state. The module must remain loaded.
MFS_INIT_API int32_t mfs_framepool_init_record(uint32_t index, uint64_t* output, size_t count) MFS_INIT_NOEXCEPT;
#ifdef __cplusplus
}
#endif
