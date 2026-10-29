// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "resource-observation.h"
#include <cstddef>
#include <limits>
// Private acceptance ABI, not an SDK header/export. Exact fixed-width scalar
// record is checked by Rust layout queries and consumed in each real probe.
extern "C" __attribute__((visibility("default")))
int32_t mfs_mac_resource_snapshot(MfsMacResources* output) noexcept {
    if(!output) { return 1; }
    *output={};
    try { *output=mfs_mac_observe_resources();return 0; } catch(...) { return 2; }
}
extern "C" __attribute__((visibility("default")))
size_t mfs_mac_resource_layout(uint32_t index) noexcept {
    constexpr size_t values[]={1,sizeof(MfsMacResources),alignof(MfsMacResources),
        offsetof(MfsMacResources,descriptors),offsetof(MfsMacResources,threads),offsetof(MfsMacResources,ports)};
    return index<sizeof(values)/sizeof(values[0]) ? values[index] : std::numeric_limits<size_t>::max();
}
