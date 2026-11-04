// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "MaaControlUnit/Win32ControlUnitAPI.h"
// Readonly diagnostics beside complete unchanged original translation units.
namespace MfsWholeGamepad {
using Outer=MAA_CTRL_UNIT_NS::GamepadControlUnitMgr;
struct CaptureMember {
    using type=std::shared_ptr<MAA_CTRL_UNIT_NS::ControlUnitAPI> Outer::*;
    friend type capture_member(CaptureMember);
};
// Typed member pointer, permitted by explicit-instantiation access rules.
// https://eel.is/c++draft/temp.spec.general#6 . No source/layout rewriting.
template<class Tag,typename Tag::type Member> struct MemberAccess {
    friend typename Tag::type capture_member(Tag) {return Member;}
};
template struct MemberAccess<CaptureMember,&Outer::win32_unit_>;
struct Trace {
    std::filesystem::path provider;
    std::ofstream rows;
    unsigned count=0;
    Trace(const std::filesystem::path& directory,const std::filesystem::path& output)
        :provider(directory/L"MaaWin32ControlUnit.dll"),rows(output/L"source-whole-chain.tsv") {}
    uint64_t record(unsigned cycle,unsigned stage,MAA_CTRL_UNIT_NS::GamepadControlUnitAPI* unit) {
        const auto* outer=unit?dynamic_cast<const Outer*>(unit):nullptr;
        if((stage==7)!=(unit==nullptr) || (unit && !outer)) {throw std::runtime_error("actual original outer class differs");}
        // Borrow the original owner, without changing any reference count.
        const auto* inner=outer?(outer->*capture_member(CaptureMember{})).get():nullptr;
        const auto module=GetModuleHandleW(provider.c_str()); // nonowning
        if((stage==0 || stage==7)!=(inner==nullptr) || (inner==nullptr)!=(module==nullptr)) {
            throw std::runtime_error("actual original inner owner/module lifetime differs");
        }
        uint64_t inspect[8]{},selected[5]{};
        if(inner) {
            using Inspect=int32_t(*)(void*,uint64_t*,size_t);
            using Version=const char*(*)();
            const auto version=reinterpret_cast<Version>(GetProcAddress(module,"MaaWin32ControlUnitGetVersion"));
            const auto type=reinterpret_cast<Inspect>(GetProcAddress(module,"mfs_whole_inspect"));
            const auto selection=reinterpret_cast<Inspect>(GetProcAddress(module,"mfs_whole_selected"));
            const auto* native=dynamic_cast<const MAA_CTRL_UNIT_NS::Win32ControlUnitAPI*>(inner);
            const auto* label=version?version():nullptr;
            if(!native || !label || !type || !selection || std::string_view(label)!="mfs-p1-whole-win32-class" ||
               type(const_cast<MAA_CTRL_UNIT_NS::Win32ControlUnitAPI*>(native),inspect,std::size(inspect)) ||
               selection(const_cast<MAA_CTRL_UNIT_NS::Win32ControlUnitAPI*>(native),selected,std::size(selected))) {
                throw std::runtime_error("actual original inner diagnostic call failed");
            }
            if(inspect[0]!=0x4d465357 || inspect[1]!=1 || inspect[2] || inspect[3] || inspect[4]!=1 ||
               inspect[5]<8 || inspect[6]!=8 || inspect[7]!=GetCurrentThreadId() ||
               selected[0]!=0x4d465353 || selected[1]!=1 || !selected[2] || !selected[3] || selected[4]!=GetCurrentThreadId()) {
                throw std::runtime_error("actual original inner class/selection/counters differ");
            }
        }
        rows<<cycle<<'\t'<<stage<<'\t'<<GetCurrentThreadId()<<'\t'<<reinterpret_cast<uintptr_t>(outer)
            <<'\t'<<reinterpret_cast<uintptr_t>(inner)<<'\t'<<reinterpret_cast<uintptr_t>(module);
        for(const auto value:inspect) {rows<<'\t'<<value;}
        for(const auto value:selected) {rows<<'\t'<<value;}
        rows<<'\t'<<(outer!=nullptr)<<'\n';rows.flush();++count;
        if(!rows) {throw std::runtime_error("whole original chain record write failed");}
        return selected[2];
    }
    void finish() {
        rows.close();if(count!=160 || !rows) {throw std::runtime_error("incomplete whole original chain observations");}
    }
};
}
