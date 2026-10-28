// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Include inside an isolated test namespace after naming capture classes.
// Only input=None is exercised; no input implementation is imported.
struct InputBase { virtual ~InputBase()=default; virtual void inactive() {} };
struct BackgroundManagedKeyInput {
    explicit BackgroundManagedKeyInput(HWND) {}
    bool inactive() { return true; }
};
struct Win32ControlUnitMgr {
#include "manager-enum.inc"
    Win32ControlUnitMgr(HWND,MaaWin32ScreencapMethod,MaaWin32InputMethod,MaaWin32InputMethod);
    bool connect(); bool connected() const; bool screencap(cv::Mat&); bool inactive();
    std::unordered_map<ScreencapMethod,std::shared_ptr<ScreencapBase>> build_screencap_units() const;
    bool init_screencap();
    std::shared_ptr<ScreencapBase> speed_test(const std::unordered_map<ScreencapMethod,std::shared_ptr<ScreencapBase>>&) const;
    std::shared_ptr<InputBase> make_input(MaaWin32InputMethod) const { return nullptr; }
    HWND hwnd_=nullptr;
    MaaWin32ScreencapMethod screencap_method_=0;
    MaaWin32InputMethod mouse_method_=0,keyboard_method_=0;
    bool connected_=false;
    std::shared_ptr<InputBase> mouse_,keyboard_;
    std::shared_ptr<ScreencapBase> screencap_;
    std::shared_ptr<BackgroundManagedKeyInput> background_keyboard_;
};
