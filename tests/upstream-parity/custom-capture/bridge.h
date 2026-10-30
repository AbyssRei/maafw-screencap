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
using MfsCustomUuid=decltype(MaaCustomControllerCallbacks::request_uuid);
using MfsCustomFeatures=decltype(MaaCustomControllerCallbacks::get_features);
using MfsCustomStringScope=int32_t(*)(void*,MaaStringBuffer*);
struct MfsCustomStringView {const uint8_t* data;size_t length;};
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
MFS_CUSTOM_EXPORT size_t mfs_custom_identity_abi(uint32_t index) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_identity_configure(void*,MfsCustomUuid,MfsCustomFeatures,void*) noexcept;
MFS_CUSTOM_EXPORT uint8_t mfs_custom_has_uuid(void*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_uuid_invoke(void*,MaaStringBuffer*,uint8_t*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_features(void*,uint8_t,uint64_t*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_string_scope(MfsCustomStringScope,void*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_string_set(MaaStringBuffer*,const uint8_t*,size_t) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_string_view(MaaStringBuffer*,MfsCustomStringView*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_uuid_output(const uint8_t*,size_t,void**) noexcept;
MFS_CUSTOM_EXPORT void mfs_custom_uuid_release(void*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_uuid_view(void*,MfsCustomStringView*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_uuid_source(void*,void*,uint8_t*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_identity_fault(void*,uint32_t) noexcept;
using MfsCustomInfo=decltype(MaaCustomControllerCallbacks::get_info);
MFS_CUSTOM_EXPORT size_t mfs_custom_info_abi(uint32_t) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_info_configure(void*,MfsCustomInfo,void*) noexcept;
MFS_CUSTOM_EXPORT uint8_t mfs_custom_has_info(void*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_info_invoke(void*,MaaStringBuffer*,uint8_t*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_info_empty(void**) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_info_parse(MaaStringBuffer*,void**) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_info_type(void*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_info_source(void*,void**) noexcept;
MFS_CUSTOM_EXPORT void mfs_custom_info_release(void*) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_info_serialize(void*,void**) noexcept;
MFS_CUSTOM_EXPORT int32_t mfs_custom_info_fault(void*) noexcept;
MFS_CUSTOM_EXPORT uint64_t mfs_custom_info_count(uint32_t) noexcept;
