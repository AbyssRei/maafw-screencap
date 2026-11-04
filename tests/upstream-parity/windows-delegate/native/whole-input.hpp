// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "Base/UnitBase.h"
#include "whole-exclusions.hpp"
MAA_CTRL_UNIT_NS_BEGIN
class MfsExcludedInput : public RelativeMoveInput {
public:
    template<class... Args> explicit MfsExcludedInput(Args&&...) {MfsWhole::input();}
    MaaControllerFeature get_features() const override {MfsWhole::input();}
    bool click(int,int) override {MfsWhole::input();}
    bool swipe(int,int,int,int,int) override {MfsWhole::input();}
    bool touch_down(int,int,int,int) override {MfsWhole::input();}
    bool touch_move(int,int,int,int) override {MfsWhole::input();}
    bool touch_up(int) override {MfsWhole::input();}
    bool click_key(int) override {MfsWhole::input();}
    bool input_text(const std::string&) override {MfsWhole::input();}
    bool key_down(int) override {MfsWhole::input();}
    bool key_up(int) override {MfsWhole::input();}
    bool scroll(int,int) override {MfsWhole::input();}
    bool relative_move(int,int) override {MfsWhole::input();}
    void inactive() override {MfsWhole::input();}
};
class SeizeInput : public MfsExcludedInput {public:using MfsExcludedInput::MfsExcludedInput;};
class LegacyEventInput : public MfsExcludedInput {public:using MfsExcludedInput::MfsExcludedInput;};
class InterceptionInput : public MfsExcludedInput {public:using MfsExcludedInput::MfsExcludedInput;};
class AnchoredTouchInput : public MfsExcludedInput {public:using MfsExcludedInput::MfsExcludedInput;};
class MessageInput : public MfsExcludedInput {
public:
    enum class Mode {SendMessage,PostMessage};
    struct Config {Mode mode=Mode::SendMessage;bool with_cursor_pos=false,with_window_pos=false,track_hardware_mouse=true,block_input=false;};
    using MfsExcludedInput::MfsExcludedInput;
    bool set_mouse_lock_follow(bool) {MfsWhole::input();}
};
MAA_CTRL_UNIT_NS_END
