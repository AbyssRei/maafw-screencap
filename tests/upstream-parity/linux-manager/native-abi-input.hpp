// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included inside the original namespace after full public declarations.
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
