// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private acceptance ABI; no SDK or production resource tracking.
#pragma once
#include <stddef.h>
#include <stdint.h>
#ifdef MFS_D3D_LIFETIME_EXPORTS
#define MFS_LIFETIME_API __declspec(dllexport)
#else
#define MFS_LIFETIME_API __declspec(dllimport)
#endif
#ifdef __cplusplus
extern "C" {
#define MFS_LIFETIME_NOEXCEPT noexcept
#else
#define MFS_LIFETIME_NOEXCEPT
#endif
MFS_LIFETIME_API uint32_t mfs_d3d_lifetime_version(void) MFS_LIFETIME_NOEXCEPT;
MFS_LIFETIME_API uint64_t mfs_d3d_lifetime_clock(void) MFS_LIFETIME_NOEXCEPT;
MFS_LIFETIME_API int32_t mfs_d3d_lifetime_widths(uint32_t*,size_t) MFS_LIFETIME_NOEXCEPT;
// Borrow one actual interface on its owning caller. kind0 device,1 immediate
// context,2 output duplication,3 staging texture. The fixture uses native D3D
// destruction notification for0/1/3 and a private-data IUnknown owner for2.
// No query/interface/object reference is retained after this call. Each index
// 0..79 is used once. No silent fallback if the D3D notifier is unavailable.
MFS_LIFETIME_API int32_t mfs_d3d_lifetime_attach(void*,uint32_t kind,uint32_t index) MFS_LIFETIME_NOEXCEPT;
// Twelve process-lifetime atomic scalars: attempted, method(1 notifier/2 private
// owner),kind,registered,callbacks,attach TID,callback TID,attach QPC,callback QPC,
// callback ID,borrowed interface address,attach HRESULT as u32 bits. Addresses
// are identities only. Callbacks never access a native object, caller state,
// allocator or stream. Keep the DLL loaded through process exit on every path.
MFS_LIFETIME_API int32_t mfs_d3d_lifetime_read(uint32_t,uint64_t*,size_t) MFS_LIFETIME_NOEXCEPT;
#ifdef __cplusplus
}
#endif
