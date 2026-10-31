// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private acceptance intervention, never a product ABI.
#pragma once
#include <stdint.h>
#define MFS_PW_MEMFD_EXPORT __attribute__((visibility("default")))
extern "C" {
// Arm only with no native stream/callback alive. Reads/disarm follow full stop.
MFS_PW_MEMFD_EXPORT int32_t mfs_pw_memfd_probe_arm() noexcept;
// Fields: successful dequeues, restored pointers, live leases, errors.
MFS_PW_MEMFD_EXPORT uint64_t mfs_pw_memfd_probe_read(uint32_t field) noexcept;
MFS_PW_MEMFD_EXPORT int32_t mfs_pw_memfd_probe_disarm() noexcept;
}
