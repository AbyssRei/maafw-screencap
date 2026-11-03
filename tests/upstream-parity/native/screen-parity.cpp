// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "screen-parity.h"
#include "gdi-probe-internal.hpp"
#include "Screencap/ScreenDCScreencap.h"
#include "Screencap/HwndUtils.hpp"
#include <dwmapi.h>
#include <algorithm>

int32_t mfs_screen_parity_ensure_geometry(void* window)
{
    try { return MAA_CTRL_UNIT_NS::ensure_window_on_screen(static_cast<HWND>(window)) ? 0 : 1; }
    catch (...) { return 2; }
}

extern "C" MFS_PARITY_EXPORT int32_t mfs_screen_parity_reference(void* window, MfsParityImage* output)
{
    if (!output) { return 1; }
    *output = {};
    try {
        MAA_CTRL_UNIT_NS::ScreenDCScreencap capture(static_cast<HWND>(window));
        auto pixels = capture.screencap();
        return pixels ? mfs_parity::transfer_image(std::move(*pixels), output) : 1;
    }
    catch (const std::bad_alloc&) { return 3; }
    catch (...) { return 2; }
}
