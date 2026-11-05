// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include MFS_WHOLE_LINUX_MANAGER_HEADER
#include "MaaControlUnit/LinuxControlUnitAPI.h"
#include "Input/NoneInput.h"
#include <tuple>
MAA_CTRL_UNIT_NS_BEGIN
#include "../linux-manager/native-abi-input.hpp"
// Complete original Wlr/client classes are already declared. These unrelated
// native/input constructors must stay unselected; no input actions are run.
class PipeWireScreencap : public ScreencapBase {
public:
    PipeWireScreencap(int,uint32_t) {throw std::logic_error("PipeWire excluded from whole Wlr caller");}
    bool screencap(cv::Mat&) override {throw std::logic_error("PipeWire excluded");}
};
class WlrInput : public FixtureInput {
public:
    explicit WlrInput(std::shared_ptr<WaylandClient>) {throw std::logic_error("Wlr input excluded");}
};
class UInput : public FixtureInput {
public:
    UInput(const std::filesystem::path&,int,int) {throw std::logic_error("UInput excluded");}
};
class EiInput : public FixtureInput {
public:
    explicit EiInput(const std::filesystem::path&) {throw std::logic_error("LibEi excluded");}
};
inline int vk_to_evdev(int) {throw std::logic_error("key translation excluded");}
MAA_CTRL_UNIT_NS_END
#include MFS_WHOLE_LINUX_MANAGER_CPP
#include "../linux-manager/whole-owner-observer.hpp"
namespace MfsWholeLinux {
struct ClientMember {
    using type=std::shared_ptr<MAA_CTRL_UNIT_NS::WaylandClient> Manager::*;
    friend type client_member(ClientMember);
};
template<class Tag,typename Tag::type Member> struct ClientAccess {
    friend typename Tag::type client_member(Tag) {return Member;}
};
template struct ClientAccess<ClientMember,&Manager::wl_client_>;
inline const auto& client(MAA_CTRL_UNIT_NS::LinuxControlUnitAPI& unit) {
    const auto* manager=dynamic_cast<const Manager*>(&unit);
    if(!manager) {throw std::logic_error("not the actual original Linux manager");}
    return manager->*client_member(ClientMember{});
}
inline bool wlr_type(MAA_CTRL_UNIT_NS::LinuxControlUnitAPI& unit) {
    const auto* manager=dynamic_cast<const Manager*>(&unit);
    if(!manager) {throw std::logic_error("not the actual original Linux manager");}
    const auto* capture=(manager->*capture_member(CaptureMember{})).get();
    return capture && typeid(*capture)==typeid(MAA_CTRL_UNIT_NS::WlrScreencap);
}
}
