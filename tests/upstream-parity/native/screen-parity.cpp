// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "screen-parity.h"
#include "gdi-probe-internal.hpp"
#include "Screencap/ScreenDCScreencap.h"
#include <dwmapi.h>

// Fixture readiness only. No product/source capture call is modified. Records
// GDI submission and DWM's presentation barrier as distinct native evidence.
extern "C" MFS_PARITY_EXPORT int32_t mfs_screen_fixture_present(int32_t* output, size_t capacity)
{
    if (!output || capacity < 2) { return 1; }
    output[0] = GdiFlush();
    output[1] = DwmFlush();
    return 0;
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
