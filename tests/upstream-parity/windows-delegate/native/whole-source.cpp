// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Diagnostics only. Actual factory, manager and empty background-key owner
// compile directly from the complete unchanged F translation units/headers.
#include "MaaControlUnit/Win32ControlUnitAPI.h"
#include "Manager/Win32ControlUnitMgr.h"
#include "whole-exclusions.hpp"
#include <algorithm>
#include <iterator>
#include <windows.h>
extern "C" __declspec(dllexport) int32_t mfs_whole_inspect(void* unit,uint64_t* output,size_t length) noexcept {
    if(!unit || !output || length!=8) {return 1;}
    const auto* actual=dynamic_cast<const MAA_CTRL_UNIT_NS::Win32ControlUnitMgr*>(static_cast<MaaWin32ControlUnitHandle>(unit));
    const uint64_t values[]{0x4d465357,1,MfsWhole::input_calls.load(),MfsWhole::application_calls.load(),actual!=nullptr,
        sizeof(MAA_CTRL_UNIT_NS::Win32ControlUnitMgr),alignof(MAA_CTRL_UNIT_NS::Win32ControlUnitMgr),GetCurrentThreadId()};
    std::copy(std::begin(values),std::end(values),output);return actual?0:2;
}
