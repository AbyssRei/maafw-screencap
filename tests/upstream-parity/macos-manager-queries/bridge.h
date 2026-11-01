// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private acceptance ABI. Fixed CPU copies only; never an SDK declaration.
#pragma once
#include <stddef.h>
#include <stdint.h>
#if defined(_WIN32)
#define MFS_MQ_EXPORT __declspec(dllexport)
#else
#define MFS_MQ_EXPORT __attribute__((visibility("default")))
#endif
struct MfsMqValue {
    uint64_t scalar;
    size_t length;
    uint8_t bytes[256];
};
struct MfsMqHost {
    // Copied table; borrowed context stays live through unit destruction.
    // Calls are synchronous on the creating thread, without reentry or retention.
    void* context;
    // 0=connect,1=connected,2=uuid,3=features,4=info,5=inactive.
    int32_t (*call)(void*,uint32_t,struct MfsMqValue*);
};
struct MfsMqSnapshot {
    // Initialized fixed CPU arrays; no C++/Rust native owner escapes.
    uint64_t features;
    uint32_t connected,uuid_success;
    size_t uuid_length,info_length;
    uint8_t uuid[11],info[256];
};
#ifdef __cplusplus
extern "C" {
#define MFS_MQ_NOEXCEPT noexcept
#else
#define MFS_MQ_NOEXCEPT
#endif
MFS_MQ_EXPORT size_t mfs_mq_layout(uint32_t) MFS_MQ_NOEXCEPT;
// 0=portable forced unavailable,1=real macOS runtime available,2=real unavailable.
MFS_MQ_EXPORT uint32_t mfs_mq_environment(void) MFS_MQ_NOEXCEPT;
MFS_MQ_EXPORT int32_t mfs_mq_create(uint32_t,uint64_t,const struct MfsMqHost*,void**) MFS_MQ_NOEXCEPT;
MFS_MQ_EXPORT int32_t mfs_mq_connect(void*,uint32_t*) MFS_MQ_NOEXCEPT;
MFS_MQ_EXPORT int32_t mfs_mq_inactive(void*,uint32_t*) MFS_MQ_NOEXCEPT;
MFS_MQ_EXPORT int32_t mfs_mq_query(void*,struct MfsMqSnapshot*) MFS_MQ_NOEXCEPT;
MFS_MQ_EXPORT int32_t mfs_mq_destroy(void*) MFS_MQ_NOEXCEPT;
#ifdef __cplusplus
}
#endif
