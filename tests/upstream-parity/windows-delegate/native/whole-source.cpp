// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Diagnostics only. Actual factory, manager and empty background-key owner
// compile directly from the complete unchanged F translation units/headers.
#include "MaaControlUnit/Win32ControlUnitAPI.h"
#include "Manager/Win32ControlUnitMgr.h"
#include "whole-exclusions.hpp"
#include "Screencap/GdiScreencap.h"
#include "Screencap/FramePoolWithPseudoMinimizeScreencap.h"
#include "Screencap/DesktopDupScreencap.h"
#include "Screencap/DesktopDupWindowScreencap.h"
#include "Screencap/PrintWindowWithPseudoMinimizeScreencap.h"
#include "Screencap/ScreenDCScreencap.h"
#include <algorithm>
#include <iterator>
#include <windows.h>
namespace {
using Manager=MAA_CTRL_UNIT_NS::Win32ControlUnitMgr;
// Test-only typed member pointer, not an offset/layout copy or shared_ptr copy.
// Access checking excludes names specifying explicit instantiations:
// https://eel.is/c++draft/temp.spec.general#6 . Original class/header unchanged.
struct SelectedMember {
    using type=std::shared_ptr<MAA_CTRL_UNIT_NS::ScreencapBase> Manager::*;
    friend type selected_member(SelectedMember);
};
template<class Tag,typename Tag::type Member> struct MemberAccess {
    friend typename Tag::type selected_member(Tag) {return Member;}
};
template struct MemberAccess<SelectedMember,&Manager::screencap_>;
uint64_t selected_kind(const MAA_CTRL_UNIT_NS::ScreencapBase& unit) noexcept {
    using namespace MAA_CTRL_UNIT_NS;
    if(typeid(unit)==typeid(GdiScreencap)) {return 1;}
    if(typeid(unit)==typeid(FramePoolWithPseudoMinimizeScreencap)) {return 2;}
    if(typeid(unit)==typeid(DesktopDupScreencap)) {return 4;}
    if(typeid(unit)==typeid(DesktopDupWindowScreencap)) {return 8;}
    if(typeid(unit)==typeid(PrintWindowWithPseudoMinimizeScreencap)) {return 16;}
    if(typeid(unit)==typeid(ScreenDCScreencap)) {return 32;}
    return 0;
}
}
extern "C" __declspec(dllexport) int32_t mfs_whole_inspect(void* unit,uint64_t* output,size_t length) noexcept {
    if(!unit || !output || length!=8) {return 1;}
    const auto* actual=dynamic_cast<const MAA_CTRL_UNIT_NS::Win32ControlUnitMgr*>(static_cast<MaaWin32ControlUnitHandle>(unit));
    const uint64_t values[]{0x4d465357,1,MfsWhole::input_calls.load(),MfsWhole::application_calls.load(),actual!=nullptr,
        sizeof(MAA_CTRL_UNIT_NS::Win32ControlUnitMgr),alignof(MAA_CTRL_UNIT_NS::Win32ControlUnitMgr),GetCurrentThreadId()};
    std::copy(std::begin(values),std::end(values),output);return actual?0:2;
}
extern "C" __declspec(dllexport) int32_t mfs_whole_selected(void* unit,uint64_t* output,size_t length) noexcept {
    if(!unit || !output || length!=5) {return 1;}
    const auto* actual=dynamic_cast<const Manager*>(static_cast<MaaWin32ControlUnitHandle>(unit));
    if(!actual) {return 2;}
    // Caller serializes this observation with operations on this live unit.
    // Borrow only: no ownership change, backend call, message pump or mutation.
    const auto& owner=actual->*selected_member(SelectedMember{});
    const auto kind=owner?selected_kind(*owner):0;
    if(owner && !kind) {return 3;}
    const uint64_t values[]{0x4d465353,1,kind,reinterpret_cast<uintptr_t>(owner.get()),GetCurrentThreadId()};
    std::copy(std::begin(values),std::end(values),output);return 0;
}
