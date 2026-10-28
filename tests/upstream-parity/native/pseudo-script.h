// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"

typedef struct MfsPseudoEvent {
    uint32_t operation;
    uint32_t error;
    int64_t argument;
    int64_t result;
} MfsPseudoEvent;
typedef struct MfsPseudoReport {
    uint32_t count;
    uint32_t overflow;
    MfsPseudoEvent events[64];
    uint64_t snapshots[4][5]; // style, alpha, pseudo, iconic, valid
} MfsPseudoReport;

#ifdef __cplusplus
extern "C" {
#endif
// Runs exact original helper bodies with scripted native calls and an explicitly
// driven fake std::thread. Proves state/call/sleep-request semantics, not OS
// scheduling, real window effects or real thread ownership. No capture runs.
MFS_PARITY_EXPORT int32_t mfs_pseudo_script(uint32_t scenario, MfsPseudoReport* output);
MFS_PARITY_EXPORT int32_t mfs_pseudo_script_layout(uint64_t* output, size_t capacity);
#ifdef __cplusplus
}
#endif
