// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// This CPU-only configuration caller compiles the complete original manager.
// These explicitly unselected native dependencies reject construction; they
// are not capture implementations. The separate None lifecycle uses the real
// original None capture/input classes without a native connection/input action.
#include "Base/UnitBase.h"
#include <filesystem>
#include <memory>
#include <stdexcept>
#include <sstream>
#include <tuple>

MAA_CTRL_UNIT_NS_BEGIN
#include "native-abi-input.hpp"
class WaylandClient {
public:
    bool connect(const std::filesystem::path&) { throw std::logic_error("native connection excluded from whole-class query"); }
};
class WlrScreencap : public ScreencapBase {
public:
    explicit WlrScreencap(std::shared_ptr<WaylandClient>) { throw std::logic_error("native Wlr excluded from whole-class query"); }
    bool screencap(cv::Mat&) override { throw std::logic_error("native capture excluded from whole-class query"); }
};
class PipeWireScreencap : public ScreencapBase {
public:
    PipeWireScreencap(int, uint32_t) { throw std::logic_error("native PipeWire excluded from whole-class query"); }
    bool screencap(cv::Mat&) override { throw std::logic_error("native capture excluded from whole-class query"); }
};
class WlrInput : public FixtureInput {
public:
    explicit WlrInput(std::shared_ptr<WaylandClient>) { throw std::logic_error("native input excluded from whole-class query"); }
};
class UInput : public FixtureInput {
public:
    UInput(const std::filesystem::path&, int, int) { throw std::logic_error("native input excluded from whole-class query"); }
};
class EiInput : public FixtureInput {
public:
    explicit EiInput(const std::filesystem::path&) { throw std::logic_error("native input excluded from whole-class query"); }
};
inline int vk_to_evdev(int) { throw std::logic_error("input key translation excluded from whole-class query"); }
MAA_CTRL_UNIT_NS_END
