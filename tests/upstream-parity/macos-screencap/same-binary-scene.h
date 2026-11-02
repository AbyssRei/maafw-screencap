// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private native fixture ABI only; no SDK entry point.
#pragma once
#include <stdint.h>
#ifdef __cplusplus
#define MFS_SCENE_NOEXCEPT noexcept
extern "C" {
#else
#define MFS_SCENE_NOEXCEPT
#endif
// mode0: existing ScreenCaptureKit metadata scene; mode1: existing CG scene.
// Both strings are NUL-terminated UTF-8 and live for the synchronous call.
__attribute__((visibility("default"))) uint32_t mfs_mac_same_binary_scene_version(void) MFS_SCENE_NOEXCEPT;
__attribute__((visibility("default"))) int32_t mfs_mac_same_binary_scene_run(
    uint32_t mode, const char* executable, const char* directory) MFS_SCENE_NOEXCEPT;
// Separate full RGB scene, both order arms use CG-only producer metadata.
// mode0/1 identifies the caller's original-first/Rust-first order only.
__attribute__((visibility("default"))) uint32_t mfs_mac_full_display_scene_version(void) MFS_SCENE_NOEXCEPT;
__attribute__((visibility("default"))) int32_t mfs_mac_full_display_scene_run(
    uint32_t mode, const char* executable, const char* directory) MFS_SCENE_NOEXCEPT;
// Separate CPU-view/window-census diagnostic; original strict scene stays available.
__attribute__((visibility("default"))) uint32_t mfs_mac_full_display_witness_version(void) MFS_SCENE_NOEXCEPT;
__attribute__((visibility("default"))) int32_t mfs_mac_full_display_witness_run(
    uint32_t mode, const char* executable, const char* directory) MFS_SCENE_NOEXCEPT;
// Independent owned-window maximum-level control with the same RGB/witness protocol.
__attribute__((visibility("default"))) uint32_t mfs_mac_maximum_level_scene_version(void) MFS_SCENE_NOEXCEPT;
__attribute__((visibility("default"))) int32_t mfs_mac_maximum_level_scene_run(
    uint32_t mode, const char* executable, const char* directory) MFS_SCENE_NOEXCEPT;
// Original caller fixture: create/capture/finish/destroy on the same caller
// thread. The context owns only its source object and returned CPU Mats; it
// does not own, cancel or prove drainage of the source's asynchronous blocks.
// The containing module must remain loaded until this process exits.
// Capture indices are exactly 0,1,2; finish writes held bytes after scene close.
// All paths are synchronous NUL-terminated UTF-8 borrows. Outputs start null.
// Return0 success,1 invalid call,2 contained native/C++/IO failure.
__attribute__((visibility("default"))) uint32_t mfs_mac_same_binary_original_version(void) MFS_SCENE_NOEXCEPT;
__attribute__((visibility("default"))) int32_t mfs_mac_same_binary_original_create(void**) MFS_SCENE_NOEXCEPT;
__attribute__((visibility("default"))) int32_t mfs_mac_same_binary_original_capture(
    void*, uint32_t call, const char* directory) MFS_SCENE_NOEXCEPT;
__attribute__((visibility("default"))) int32_t mfs_mac_same_binary_original_finish(void*, const char* directory) MFS_SCENE_NOEXCEPT;
__attribute__((visibility("default"))) int32_t mfs_mac_same_binary_original_destroy(void*) MFS_SCENE_NOEXCEPT;
__attribute__((visibility("default"))) uint64_t mfs_mac_same_binary_original_owners(void) MFS_SCENE_NOEXCEPT;
// Present only in the independently instrumented sample-observation module.
__attribute__((visibility("default"))) uint32_t mfs_mac_native_sample_observer_version(void) MFS_SCENE_NOEXCEPT;
#ifdef __cplusplus
}
#endif
