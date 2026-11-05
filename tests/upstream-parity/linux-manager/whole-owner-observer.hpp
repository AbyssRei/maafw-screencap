// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Include after the original manager and None class declarations. The caller
// serializes observations with manager operations. Borrow shared_ptr slots
// without copying or changing owners; returned integers are diagnostic values,
// never a permission to access the unit after original destruction.
#include <cstdint>
#include <memory>
#include <stdexcept>
#include <typeinfo>
namespace MfsWholeLinux {
using Manager=MAA_CTRL_UNIT_NS::LinuxControlUnitMgr;
struct CaptureMember {
    using type=std::shared_ptr<MAA_CTRL_UNIT_NS::ScreencapBase> Manager::*;
    friend type capture_member(CaptureMember);
};
struct InputMember {
    using type=std::shared_ptr<MAA_CTRL_UNIT_NS::InputBase> Manager::*;
    friend type input_member(InputMember);
};
// Test-only typed member pointers, as in the Win32 original-class observer.
// Explicit instantiations do not check these private member names:
// https://eel.is/c++draft/temp.spec.general#6 . No original header/source edit.
template<class Tag,typename Tag::type Member> struct CaptureAccess {
    friend typename Tag::type capture_member(Tag) {return Member;}
};
template<class Tag,typename Tag::type Member> struct InputAccess {
    friend typename Tag::type input_member(Tag) {return Member;}
};
template struct CaptureAccess<CaptureMember,&Manager::screencap_>;
template struct InputAccess<InputMember,&Manager::input_>;
struct Owners {
    uintptr_t manager=0,capture=0,input=0;
    unsigned capture_none=0,input_none=0;
    bool operator==(const Owners&) const=default;
};
inline Owners observe(MAA_CTRL_UNIT_NS::LinuxControlUnitAPI& unit) {
    const auto* actual=dynamic_cast<const Manager*>(&unit);
    if(!actual || typeid(*actual)!=typeid(Manager)) {throw std::logic_error("not the actual original Linux manager class");}
    const auto& capture=actual->*capture_member(CaptureMember{});
    const auto& input=actual->*input_member(InputMember{});
    return {reinterpret_cast<uintptr_t>(actual),reinterpret_cast<uintptr_t>(capture.get()),reinterpret_cast<uintptr_t>(input.get()),
        static_cast<unsigned>(capture && typeid(*capture)==typeid(MAA_CTRL_UNIT_NS::NoneScreencap)),
        static_cast<unsigned>(input && typeid(*input)==typeid(MAA_CTRL_UNIT_NS::NoneInput))};
}
}
