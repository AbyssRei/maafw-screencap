// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Full unchanged public API/manager/config/base declarations. Only unrelated
// native/input boundaries throw; screenshot bodies use the real original PW.
#include "Manager/LinuxControlUnitMgr.h"
#include "MaaControlUnit/KWinControlUnitAPI.h"
#include "MaaToolkit/Portal/MaaToolkitPortal.h"
MAA_CTRL_UNIT_NS_BEGIN
class FixtureInput : public InputBase {
public:
    MaaControllerFeature get_features() const override {return 0;}
    bool click(int,int) override {throw std::logic_error("input actions excluded");}
    bool swipe(int,int,int,int,int) override {throw std::logic_error("input actions excluded");}
    bool touch_down(int,int,int,int) override {throw std::logic_error("input actions excluded");}
    bool touch_move(int,int,int,int) override {throw std::logic_error("input actions excluded");}
    bool touch_up(int) override {throw std::logic_error("input actions excluded");}
    bool click_key(int) override {throw std::logic_error("input actions excluded");}
    bool input_text(const std::string&) override {throw std::logic_error("input actions excluded");}
    bool key_down(int) override {throw std::logic_error("input actions excluded");}
    bool key_up(int) override {throw std::logic_error("input actions excluded");}
    bool scroll(int,int) override {throw std::logic_error("input actions excluded");}
};
class NoneInput : public FixtureInput {};
class WaylandClient {
public:
    bool connect(const std::filesystem::path&) {throw std::logic_error("Wayland excluded from KWin/PipeWire context");}
};
class WlrScreencap : public ScreencapBase {
public:
    explicit WlrScreencap(std::shared_ptr<WaylandClient>) {throw std::logic_error("Wlr excluded from KWin/PipeWire context");}
    bool screencap(cv::Mat&) override {throw std::logic_error("Wlr excluded");}
};
class WlrInput : public FixtureInput {
public:
    explicit WlrInput(std::shared_ptr<WaylandClient>) {throw std::logic_error("Wlr input excluded");}
};
class EiInput : public FixtureInput {
public:
    explicit EiInput(const std::filesystem::path&) {throw std::logic_error("LibEi input excluded");}
};
#define MFS_HOST_INPUT_BASE FixtureInput
#include "../linux-manager/host-input-control.hpp"
#undef MFS_HOST_INPUT_BASE
#include "linux-manager-source.inc"
#include "linux-manager-features.inc"
#include "linux-manager-info.inc"
// These are the full original virtual slots, required by the real ABI owner.
// None of them is exercised as an input implementation by this capture fixture.
bool LinuxControlUnitMgr::start_app(const std::string&) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::stop_app(const std::string&) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::click(int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::swipe(int,int,int,int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::touch_down(int,int,int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::touch_move(int,int,int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::touch_up(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::click_key(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::input_text(const std::string&) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::key_down(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::key_up(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::scroll(int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::relative_move(int,int) {throw std::logic_error("input excluded");}
MAA_CTRL_UNIT_NS_END
namespace MfsOriginalPipeWire=MAA_CTRL_UNIT_NS;
