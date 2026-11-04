// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private controlled manager-body model. No native input actions or SDK ABI.
#pragma once
#include <stddef.h>
#include <stdint.h>
#if defined(_WIN32)
#define MFS_MH_EXPORT __declspec(dllexport)
#else
#define MFS_MH_EXPORT __attribute__((visibility("default")))
#endif
struct MfsMhScript {
    uint64_t method,input_method;
    uint32_t window_id,available[2],fail_capture[2],fail_input[2];
    uint64_t features[2];
    uint32_t outcomes[3]; // nullopt, pixel, empty, exception.
};
struct MfsMhEvent {uint32_t kind,unit,phase,call;};
struct MfsMhReport {
    uint32_t connect[2],connected[3],capture_owner[2],input_owner[2];
    uint64_t features[4]; // cold, first connect, inactive, second connect.
    uint32_t captures[4],pixels[4];
    uint32_t capture_creates,input_creates,calls,queries,event_count,overflow;
    struct MfsMhEvent events[32];
    uint32_t info_length;
    uint8_t info[256];
};
#ifdef __cplusplus
extern "C" {
MFS_MH_EXPORT int32_t mfs_mh_run(const MfsMhScript*,MfsMhReport*) noexcept;
MFS_MH_EXPORT size_t mfs_mh_layout(uint32_t) noexcept;
}
#endif
