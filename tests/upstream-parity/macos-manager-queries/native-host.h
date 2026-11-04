// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private native acceptance entry. It constructs only modeled input owners;
// the capture class, version predicate and selected manager bodies are original.
#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
__attribute__((visibility("default")))
int32_t mfs_mh_native_run(uint32_t window,const char* fresh_directory) noexcept;
}
#endif
