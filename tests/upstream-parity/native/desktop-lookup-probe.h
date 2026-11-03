// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <stddef.h>
#include <stdint.h>
#if defined(MFS_DD_LOOKUP_EXPORTS)
#define MFS_DD_LOOKUP_API __declspec(dllexport)
#else
#define MFS_DD_LOOKUP_API __declspec(dllimport)
#endif

// Private finite lookup/fallback diagnostic, not a product/SDK ABI.
// Modes: first lookup EnumAdapters, EnumOutputs, GetDesc or output1 QI error;
// mode4 forwards GetDesc then substitutes a null monitor (controlled mismatch).
// control1 forwards the matching first output. All DuplicateOutput calls stop
// with E_ABORT; explicit enumeration tails return DXGI_ERROR_NOT_FOUND without
// consulting further physical adapters/outputs. No multi-output coverage claim.
enum MfsDdLookupField {
    MFS_LOOKUP_MODE, MFS_LOOKUP_CONTROL, MFS_LOOKUP_CALLER, MFS_LOOKUP_VIOLATION,
    MFS_LOOKUP_DEVICES, MFS_LOOKUP_FACTORIES,
    MFS_LOOKUP_ENUM_ADAPTERS, MFS_LOOKUP_NATIVE_ADAPTERS,
    MFS_LOOKUP_ENUM_OUTPUTS, MFS_LOOKUP_NATIVE_OUTPUTS,
    MFS_LOOKUP_DESC, MFS_LOOKUP_NATIVE_DESC, MFS_LOOKUP_QUERY, MFS_LOOKUP_NATIVE_QUERY,
    MFS_LOOKUP_DUPLICATE, MFS_LOOKUP_NATIVE_DUPLICATE,
    MFS_LOOKUP_FACTORY_CREATED, MFS_LOOKUP_FACTORY_DESTROYED,
    MFS_LOOKUP_ADAPTER_CREATED, MFS_LOOKUP_ADAPTER_DESTROYED,
    MFS_LOOKUP_OUTPUT_CREATED, MFS_LOOKUP_OUTPUT_DESTROYED,
    MFS_LOOKUP_FAULTS, MFS_LOOKUP_FAULT_HRESULT, MFS_LOOKUP_SENTINEL_HRESULT,
    MFS_LOOKUP_FALLBACKS, MFS_LOOKUP_BEFORE_ADAPTER_CREATED, MFS_LOOKUP_BEFORE_ADAPTER_DESTROYED,
    MFS_LOOKUP_BEFORE_OUTPUT_CREATED, MFS_LOOKUP_BEFORE_OUTPUT_DESTROYED,
    MFS_LOOKUP_ADAPTER_ENDS, MFS_LOOKUP_OUTPUT_ENDS,
    MFS_LOOKUP_DEVICE_POINTER, MFS_LOOKUP_CONTEXT_POINTER, MFS_LOOKUP_TARGET_MONITOR,
    MFS_LOOKUP_FACTORY_POINTER, MFS_LOOKUP_ADAPTER_POINTER, MFS_LOOKUP_OUTPUT_POINTER,
    MFS_LOOKUP_FIELDS
};
#ifdef __cplusplus
extern "C" {
#define MFS_LOOKUP_NOEXCEPT noexcept
#else
#define MFS_LOOKUP_NOEXCEPT
#endif
// Seven scalar fields: HWND, selected real output monitor, caller/owner TID,
// visibility, IsWindow, actual MonitorFromWindow (zero after checked close).
MFS_DD_LOOKUP_API int32_t mfs_dd_lookup_target_create(uint64_t*,size_t) MFS_LOOKUP_NOEXCEPT;
MFS_DD_LOOKUP_API int32_t mfs_dd_lookup_target_close(uint64_t*,size_t) MFS_LOOKUP_NOEXCEPT;
MFS_DD_LOOKUP_API int32_t mfs_dd_lookup_begin(uint32_t,uint32_t,uint32_t) MFS_LOOKUP_NOEXCEPT;
MFS_DD_LOOKUP_API int32_t mfs_dd_lookup_created(void*,void*) MFS_LOOKUP_NOEXCEPT;
MFS_DD_LOOKUP_API int32_t mfs_dd_lookup_factory(void*,void**) MFS_LOOKUP_NOEXCEPT;
MFS_DD_LOOKUP_API int32_t mfs_dd_lookup_trace(uint64_t*,size_t) MFS_LOOKUP_NOEXCEPT;
MFS_DD_LOOKUP_API int32_t mfs_dd_lookup_end(void) MFS_LOOKUP_NOEXCEPT;
#ifdef __cplusplus
}
#endif
