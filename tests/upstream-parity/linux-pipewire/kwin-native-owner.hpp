// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Full unchanged public API/manager/config/base declarations. Only unrelated
// native/input boundaries throw; screenshot bodies use the real original PW.
#include "Manager/LinuxControlUnitMgr.h"
#include "MaaControlUnit/KWinControlUnitAPI.h"
#include "MaaToolkit/Portal/MaaToolkitPortal.h"
MAA_CTRL_UNIT_NS_BEGIN
#include "../linux-manager/native-abi-input.hpp"
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
#include "../linux-manager/native-abi-actions.hpp"
MAA_CTRL_UNIT_NS_END
namespace MfsOriginalPipeWire=MAA_CTRL_UNIT_NS;
