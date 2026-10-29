// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Explicit private acceptance ABI, never an SDK header/export.
#pragma once
#include <stddef.h>
#include <stdint.h>
struct MfsMacManagerScript {
    uint64_t method;
    uint32_t window_id;
    uint32_t available[2];
    uint32_t fail_create[2];
    uint32_t outcomes[3]; // 0=nullopt,1=pixels,2=engaged empty,3=exception.
};
struct MfsMacManagerEvent { uint32_t kind,unit,call,reserved; };
struct MfsMacManagerReport {
    uint32_t connect_status[2]; // false=0,true=1,exception=2.
    uint32_t connected[3];
    uint32_t selected[2];
    uint32_t capture_status[4];
    uint32_t output_tokens[4];
    uint32_t inactive,uuid_success,uuid_length;
    char uuid[11];
    uint8_t reserved;
    uint32_t creates,calls,input_attempts,overflow,event_count;
    struct MfsMacManagerEvent events[16]; // create=1,capture=2,unit drop=3.
};
#ifdef __cplusplus
#define MFS_MAC_MANAGER_NOEXCEPT noexcept
extern "C" {
#else
#define MFS_MAC_MANAGER_NOEXCEPT
#endif
__attribute__((visibility("default"))) int32_t mfs_mac_manager_script(
    const struct MfsMacManagerScript*,struct MfsMacManagerReport*) MFS_MAC_MANAGER_NOEXCEPT;
__attribute__((visibility("default"))) size_t mfs_mac_manager_layout(uint32_t) MFS_MAC_MANAGER_NOEXCEPT;
#ifdef __cplusplus
}
#endif
