// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Explicit host owner, NEVER a ViGEm implementation or input/device connection.
#include "gamepad-host-fixture.hpp"
#include "Common/Conf.h"
#include <stdexcept>
MAA_CTRL_UNIT_NS_BEGIN
class ViGEmInput {
    unsigned id_=0;
    bool connected_=false;
public:
    explicit ViGEmInput(MaaGamepadType type) {
        MfsDelegateFixture::event("input.create:"+std::to_string(type));
        if(MfsGamepadFixture::mode==3) {throw std::runtime_error("host create");}
        id_=++MfsGamepadFixture::next_input;MfsGamepadFixture::input_live(id_);
        MfsDelegateFixture::event("input.new:"+std::to_string(id_));
    }
    ~ViGEmInput() {MfsDelegateFixture::event("input.drop:"+std::to_string(id_));MfsGamepadFixture::input_drop(id_);}
    bool connect() {
        MfsDelegateFixture::event("input.connect:"+std::to_string(id_));
        if(MfsGamepadFixture::mode==2) {throw std::runtime_error("host connect");}
        connected_=MfsGamepadFixture::mode!=1;return connected_;
    }
    bool connected() const {
        MfsDelegateFixture::event("input.connected:"+std::to_string(id_));
        return connected_ && MfsGamepadFixture::mode!=4;
    }
    void reset() {MfsDelegateFixture::event("input.reset:"+std::to_string(id_));}
    // The whole original manager compiles these unrelated bodies, but calling
    // one is a hard fixture failure. No foreground or device action is allowed.
    bool set_left_stick(int,int) {throw std::logic_error("input excluded");}
    bool set_right_stick(int,int) {throw std::logic_error("input excluded");}
    bool set_left_trigger(int) {throw std::logic_error("input excluded");}
    bool set_right_trigger(int) {throw std::logic_error("input excluded");}
    bool reset_left_stick() {throw std::logic_error("input excluded");}
    bool reset_right_stick() {throw std::logic_error("input excluded");}
    bool reset_left_trigger() {throw std::logic_error("input excluded");}
    bool reset_right_trigger() {throw std::logic_error("input excluded");}
    bool press_button(int) {throw std::logic_error("input excluded");}
    bool release_button(int) {throw std::logic_error("input excluded");}
};
MAA_CTRL_UNIT_NS_END
