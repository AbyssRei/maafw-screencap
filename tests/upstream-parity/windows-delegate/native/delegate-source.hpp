// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "MaaControlUnit/Win32ControlUnitAPI.h"
#include "original-LibraryHolder.h"
#include <windows.h>
#include <memory>
namespace MaaNS {
inline std::filesystem::path directory;
inline std::filesystem::path library_dir() { return directory; }
}
namespace Source {
using namespace MaaNS;
using namespace MAA_CTRL_UNIT_NS;
#include "delegate-loader.inc"
struct GamepadControlUnitMgr {
    bool init_win32_unit();bool screencap(cv::Mat&);bool inactive();
    HWND hwnd_=nullptr;
    MaaWin32ScreencapMethod screencap_method_=MaaWin32ScreencapMethod_GDI;
    std::unique_ptr<Win32ControlUnitLoader> win32_loader_;
    std::shared_ptr<ControlUnitAPI> win32_unit_;
};
#include "delegate-source.inc"
}
