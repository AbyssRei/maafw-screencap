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

int32_t mfs_screen_window_observation(void* raw_window, uint64_t* output, size_t capacity)
{
    if (!output || capacity < 16) { return 1; }
    std::fill_n(output, 16, 0);
    const HWND window = static_cast<HWND>(raw_window);
    RECT client {};
    POINT origin {};
    if (!GetClientRect(window, &client) || !ClientToScreen(window, &origin)) { return 1; }
    const int64_t width = static_cast<int64_t>(client.right) - client.left;
    const int64_t height = static_cast<int64_t>(client.bottom) - client.top;
    const int64_t x = static_cast<int64_t>(origin.x) + width / 2;
    const int64_t y = static_cast<int64_t>(origin.y) + height / 2;
    if (x < INT32_MIN || x > INT32_MAX || y < INT32_MIN || y > INT32_MAX) { return 1; }
    const HWND at_point = WindowFromPoint({ static_cast<LONG>(x), static_cast<LONG>(y) });
    DWORD cloaked = 0;
    const HRESULT code = DwmGetWindowAttribute(window, DWMWA_CLOAKED, &cloaked, sizeof(cloaked));
    const uint64_t values[16] { static_cast<uint64_t>(IsWindow(window)), static_cast<uint64_t>(IsWindowVisible(window)),
        static_cast<uint64_t>(IsIconic(window)), reinterpret_cast<uintptr_t>(GetForegroundWindow()),
        reinterpret_cast<uintptr_t>(window), reinterpret_cast<uintptr_t>(at_point),
        reinterpret_cast<uintptr_t>(GetAncestor(at_point, GA_ROOT)), reinterpret_cast<uintptr_t>(GetAncestor(window, GA_ROOT)),
        static_cast<uint64_t>(static_cast<int64_t>(code)), cloaked,
        static_cast<uint64_t>(static_cast<int64_t>(origin.x)), static_cast<uint64_t>(static_cast<int64_t>(origin.y)),
        static_cast<uint64_t>(width), static_cast<uint64_t>(height), static_cast<uint64_t>(GetWindowLongPtrW(window, GWL_EXSTYLE)), GetTickCount() };
    std::copy_n(values, 16, output);
    return 0;
}

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
