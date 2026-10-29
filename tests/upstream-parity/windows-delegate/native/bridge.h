// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <cstdint>
#include <cstddef>
#ifdef MFS_BRIDGE_EXPORTS
#define MFS_DN __declspec(dllexport)
#else
#define MFS_DN __declspec(dllimport)
#endif
struct MfsDnImage {
    void* owner;
    const uint8_t* data;
    size_t length;
    int32_t width,height;
    size_t stride;
};
// Fixed allocation-independent diagnostic buffer, outside the provider DLL.
struct MfsDnEvent { uint32_t kind,thread; uint64_t a,b,c,d; };
// Acceptance-only synchronous Rust manager facade. capture returns a borrowed
// initialized span valid until the next operation on that context. It is
// copied by the provider BEFORE the callback context can be destroyed.
struct MfsDnHost {
    void* user;
    int32_t (*create)(void*,uintptr_t,uint64_t,void**) noexcept;
    int32_t (*destroy)(void*,void*) noexcept;
    int32_t (*connect)(void*,void*,int32_t*) noexcept;
    int32_t (*capture)(void*,void*,MfsDnImage*,int32_t*) noexcept;
    int32_t (*inactive)(void*,void*,int32_t*) noexcept;
};
extern "C" {
MFS_DN int32_t mfs_dn_create(uintptr_t,uintptr_t,uint64_t,uint64_t,uint64_t,void**) noexcept;
MFS_DN int32_t mfs_dn_destroy(uintptr_t,void*) noexcept;
MFS_DN int32_t mfs_dn_connect(void*,int32_t*) noexcept;
MFS_DN int32_t mfs_dn_capture(void*,MfsDnImage*,int32_t*) noexcept;
MFS_DN int32_t mfs_dn_inactive(void*,int32_t*) noexcept;
MFS_DN void mfs_dn_release(void*) noexcept;
MFS_DN int32_t mfs_dn_events(MfsDnEvent*,size_t,size_t*,uint64_t*) noexcept;
MFS_DN int32_t mfs_dn_reset() noexcept;
MFS_DN void mfs_dn_event(uint32_t,uint64_t,uint64_t,uint64_t,uint64_t) noexcept;
// Provider fault injection is explicit; regular GDI/native loading is real.
MFS_DN uint32_t mfs_dn_mode() noexcept;
MFS_DN void mfs_dn_set_mode(uint32_t) noexcept;
MFS_DN int32_t mfs_dn_layout(size_t*,size_t) noexcept;
MFS_DN int32_t mfs_dn_host_install(const MfsDnHost*,size_t) noexcept;
MFS_DN int32_t mfs_dn_host_create(uintptr_t,uint64_t,void**) noexcept;
MFS_DN int32_t mfs_dn_host_destroy(void*) noexcept;
MFS_DN int32_t mfs_dn_host_connect(void*,int32_t*) noexcept;
MFS_DN int32_t mfs_dn_host_capture(void*,MfsDnImage*,int32_t*) noexcept;
MFS_DN int32_t mfs_dn_host_inactive(void*,int32_t*) noexcept;
}
