// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <cstddef>
#include <cstdint>
#include "MaaFramework/Instance/MaaCustomController.h"
#include "MaaFramework/Utility/MaaBuffer.h"
#ifdef _WIN32
#define MFS_CUSTOM_EXPORT extern "C" __declspec(dllexport)
#else
#define MFS_CUSTOM_EXPORT extern "C" __attribute__((visibility("default")))
#endif
using MfsCustomBool=decltype(MaaCustomControllerCallbacks::connect);
using MfsCustomShot=decltype(MaaCustomControllerCallbacks::screencap);
using MfsCustomScope=int32_t(*)(void*,MaaImageBuffer*);
struct MfsCustomView {
    const uint8_t* data;
    size_t length,stride;
    int32_t width,height,type,channels,references;
};
MFS_CUSTOM_EXPORT size_t mfs_custom_abi(uint32_t index) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_create(uint8_t table,void** output) noexcept;
MFS_CUSTOM_EXPORT void mfs_custom_destroy(void* context) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_configure(void* context,MfsCustomBool connect,
    MfsCustomBool connected,MfsCustomBool inactive,MfsCustomShot shot,void* argument) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_boolean(void* context,uint32_t operation,uint8_t source,uint8_t* value) noexcept;
MFS_CUSTOM_EXPORT uint8_t mfs_custom_has_capture(void* context) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_invoke(void* context,MaaImageBuffer* buffer,uint8_t* result) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_scope(MfsCustomScope action,void* argument) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_raw_set(MaaImageBuffer* buffer,void* data,size_t length,
    int32_t width,int32_t height,int32_t type,uint8_t* result) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_image(void* buffer,void** output) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_seed(void** output) noexcept;
MFS_CUSTOM_EXPORT void mfs_custom_release(void* image) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_view(void* image,MfsCustomView* output) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_source(void* context,void* image,uint8_t* result) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_fault(void* context) noexcept;
MFS_CUSTOM_EXPORT uint64_t mfs_custom_count(uint32_t kind) noexcept;
