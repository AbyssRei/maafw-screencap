// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Real original PipeWire is already defined by its complete included cpp.
// Only the explicitly unselected native/input dependencies reject construction.
#include MFS_WHOLE_LINUX_MANAGER_HEADER
#include "MaaControlUnit/LinuxControlUnitAPI.h"
#include "Input/NoneInput.h"
#include <tuple>
MAA_CTRL_UNIT_NS_BEGIN
#include "../linux-manager/native-abi-input.hpp"
class WaylandClient {
public:
    bool connect(const std::filesystem::path&) {throw std::logic_error("Wayland excluded from whole PipeWire caller");}
};
class WlrScreencap : public ScreencapBase {
public:
    explicit WlrScreencap(std::shared_ptr<WaylandClient>) {throw std::logic_error("Wlr excluded from whole PipeWire caller");}
    bool screencap(cv::Mat&) override {throw std::logic_error("Wlr capture excluded");}
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
inline int vk_to_evdev(int) {throw std::logic_error("input key translation excluded");}
MAA_CTRL_UNIT_NS_END
#include MFS_WHOLE_LINUX_MANAGER_CPP
#include "../linux-manager/whole-owner-observer.hpp"
namespace MfsWholeLinux {
inline bool pipewire_type(MAA_CTRL_UNIT_NS::LinuxControlUnitAPI& unit) {
    const auto* manager=dynamic_cast<const Manager*>(&unit);
    if(!manager) {throw std::logic_error("whole PipeWire caller lost actual manager");}
    const auto& capture=manager->*capture_member(CaptureMember{});
    return capture && typeid(*capture)==typeid(MAA_CTRL_UNIT_NS::PipeWireScreencap);
}
}
namespace MfsOriginalPipeWire=MAA_CTRL_UNIT_NS;
