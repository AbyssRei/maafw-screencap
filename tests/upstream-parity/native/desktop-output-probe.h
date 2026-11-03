// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <stddef.h>
#include <stdint.h>
#if defined(MFS_DD_OUTPUT_EXPORTS)
#define MFS_DD_OUTPUT_API __declspec(dllexport)
#else
#define MFS_DD_OUTPUT_API __declspec(dllimport)
#endif

// Private primary-output failure oracle, never a product or public SDK ABI.
// One caller thread, forty once-only contexts; native owners end before end().
// mode 0/1/2/3: EnumAdapters/EnumOutputs/output1 QueryInterface/DuplicateOutput.
// control=1 forwards real primary enumeration/query, then stops DuplicateOutput
// with explicit E_ABORT. Both public paths reject before any sample/driver dup.
enum MfsDdOutputField {
    MFS_DD_MODE, MFS_DD_CONTROL, MFS_DD_CALLER, MFS_DD_VIOLATION,
    MFS_DD_DEVICES, MFS_DD_FACTORIES, MFS_DD_ENUM_ADAPTERS, MFS_DD_NATIVE_ADAPTERS,
    MFS_DD_ENUM_OUTPUTS, MFS_DD_NATIVE_OUTPUTS, MFS_DD_QUERY, MFS_DD_NATIVE_QUERY,
    MFS_DD_DUPLICATE, MFS_DD_NATIVE_DUPLICATE,
    MFS_DD_FACTORY_CREATED, MFS_DD_FACTORY_DESTROYED,
    MFS_DD_ADAPTER_CREATED, MFS_DD_ADAPTER_DESTROYED,
    MFS_DD_OUTPUT_CREATED, MFS_DD_OUTPUT_DESTROYED,
    MFS_DD_INJECTED_HRESULT, MFS_DD_DEVICE_POINTER, MFS_DD_CONTEXT_POINTER,
    MFS_DD_FACTORY_POINTER, MFS_DD_ADAPTER_POINTER, MFS_DD_OUTPUT_POINTER,
    MFS_DD_FIELDS
};
#ifdef __cplusplus
extern "C" {
#define MFS_DD_OUTPUT_NOEXCEPT noexcept
#else
#define MFS_DD_OUTPUT_NOEXCEPT
#endif
MFS_DD_OUTPUT_API int32_t mfs_dd_output_begin(uint32_t mode,uint32_t control,uint32_t index) MFS_DD_OUTPUT_NOEXCEPT;
MFS_DD_OUTPUT_API int32_t mfs_dd_output_created(void* device,void* context) MFS_DD_OUTPUT_NOEXCEPT;
// Borrows one real factory; success transfers one facade reference to *output.
MFS_DD_OUTPUT_API int32_t mfs_dd_output_factory(void* factory,void** output) MFS_DD_OUTPUT_NOEXCEPT;
MFS_DD_OUTPUT_API int32_t mfs_dd_output_trace(uint64_t* output,size_t capacity) MFS_DD_OUTPUT_NOEXCEPT;
MFS_DD_OUTPUT_API int32_t mfs_dd_output_end(void) MFS_DD_OUTPUT_NOEXCEPT;
#ifdef __cplusplus
}
#endif
