// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Compile exact, unpatched headers with a synchronous observation boundary.
#include "window-script.h"
#include "MaaUtils/SafeWindows.hpp"
#include "MaaUtils/Logger.h"
#include "MaaUtils/NoWarningCV.hpp"
#include "Common/Conf.h"
#include <algorithm>
#include <array>
#include <chrono>
#include <thread>
#include <utility>

namespace {
thread_local const MfsWindowApi* active = nullptr;
thread_local bool violation = false;
int64_t call(uint32_t op, void* window, std::array<int64_t, 6>& args)
{ return active->invoke(active->context, op, window, args.data()); }
int64_t query(uint32_t op, void* window = nullptr)
{ std::array<int64_t, 6> args {}; return call(op, window, args); }
BOOL valid(HWND w) { return static_cast<BOOL>(query(1, w)); }
HWND foreground() { return reinterpret_cast<HWND>(query(2)); }
BOOL request(HWND w) { return static_cast<BOOL>(query(3, w)); }
DWORD ticks() { return static_cast<DWORD>(query(5)); }
BOOL zoomed(HWND w) { return static_cast<BOOL>(query(7, w)); }
BOOL iconic(HWND w) { return static_cast<BOOL>(query(8, w)); }
HMONITOR script_monitor(HWND w, DWORD flags)
{ if (flags != MONITOR_DEFAULTTONEAREST) { violation = true; } return reinterpret_cast<HMONITOR>(query(9, w)); }
BOOL monitor_info(HMONITOR m, MONITORINFO* output)
{
    if (output->cbSize != sizeof(MONITORINFO)) { violation = true; }
    std::array<int64_t, 6> a {};
    const auto result = call(10, m, a);
    output->rcWork = { static_cast<LONG>(a[0]), static_cast<LONG>(a[1]), static_cast<LONG>(a[2]), static_cast<LONG>(a[3]) };
    // rcMonitor deliberately differs from rcWork. U must select rcWork.
    output->rcMonitor = { -9999, -9999, 9999, 9999 };
    return static_cast<BOOL>(result);
}
BOOL rectangle(uint32_t op, HWND w, RECT* output)
{
    std::array<int64_t, 6> a {};
    const auto result = call(op, w, a);
    *output = { static_cast<LONG>(a[0]), static_cast<LONG>(a[1]), static_cast<LONG>(a[2]), static_cast<LONG>(a[3]) };
    return static_cast<BOOL>(result);
}
BOOL script_window_rect(HWND w, RECT* r) { return rectangle(11, w, r); }
BOOL script_client_rect(HWND w, RECT* r) { return rectangle(12, w, r); }
BOOL origin(HWND w, POINT* p)
{
    std::array<int64_t, 6> a { p->x, p->y };
    const auto result = call(13, w, a);
    *p = { static_cast<LONG>(a[0]), static_cast<LONG>(a[1]) };
    return static_cast<BOOL>(result);
}
BOOL position(HWND w, HWND after, int x, int y, int width, int height, UINT flags)
{
    if (flags == (SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS)) {
        if (after) { violation = true; }
        std::array<int64_t, 6> a { x, y, width, height };
        return static_cast<BOOL>(call(14, w, a));
    }
    const bool show = (flags & SWP_SHOWWINDOW) != 0;
    if (x || y || width || height || flags != (SWP_NOMOVE | SWP_NOSIZE | (show ? SWP_SHOWWINDOW : 0))) { violation = true; }
    const int order = after == HWND_TOP ? 0 : after == HWND_TOPMOST ? 1 : after == HWND_NOTOPMOST ? 2 : -1;
    if (order < 0) { violation = true; }
    std::array<int64_t, 6> a { order, show };
    return static_cast<BOOL>(call(4, w, a));
}
UINT dpi(HWND w) { return static_cast<UINT>(query(15, w)); }
LONG_PTR style(HWND w, int index)
{ if (index != GWL_STYLE) { violation = true; } return static_cast<LONG_PTR>(query(16, w)); }
}
namespace MfsWindowScriptStd {
namespace chrono = std::chrono;
namespace this_thread {
template<class T> void sleep_for(T duration) {
    std::array<int64_t, 6> a { std::chrono::duration_cast<std::chrono::milliseconds>(duration).count() };
    call(6, nullptr, a);
}
}
}

#undef MAA_CTRL_UNIT_NS_BEGIN
#undef MAA_CTRL_UNIT_NS_END
#define MAA_CTRL_UNIT_NS_BEGIN namespace MfsWindowReference {
#define MAA_CTRL_UNIT_NS_END }
#define IsWindow valid
#define GetForegroundWindow foreground
#define SetForegroundWindow request
#define SetWindowPos position
#define GetTickCount ticks
#define std MfsWindowScriptStd
#include "Base/ForegroundUtils.h"
#undef std
#define IsZoomed zoomed
#define IsIconic iconic
#define MonitorFromWindow script_monitor
#undef GetMonitorInfo
#define GetMonitorInfo monitor_info
#define GetWindowRect script_window_rect
#define GetClientRect script_client_rect
#define ClientToScreen origin
#define GetDpiForWindow dpi
#undef GetWindowLongPtr
#define GetWindowLongPtr style
#include "Screencap/HwndUtils.hpp"

int32_t mfs_window_script(const MfsWindowApi* api, uint32_t mode, void* window, MfsWindowResult* output)
{
    if (!output) { return 1; }
    *output = {};
    if (!api || !api->invoke || active || mode > 5) { return 1; }
    active = api;
    violation = false;
    try {
        const auto w = static_cast<HWND>(window);
        switch (mode) {
        case 0: output->first = MfsWindowReference::ensure_foreground_with_cooldown(w); break;
        case 1: MfsWindowReference::ensure_foreground_and_topmost(w); break;
        case 2: output->first = MfsWindowReference::ensure_window_on_screen(w); break;
        case 3: {
            const auto size = MfsWindowReference::window_size(w);
            output->first = size.first; output->second = size.second; break;
        }
        case 4: output->scale = MfsWindowReference::window_scale(w); break;
        case 5: output->first = MfsWindowReference::is_fullscreen(w); break;
        }
        active = nullptr;
        return violation ? 2 : 0;
    }
    catch (...) { active = nullptr; return 3; }
}
int32_t mfs_window_script_layout(uint64_t* output, size_t capacity)
{
    constexpr std::array<uint64_t, 10> layout {
        sizeof(MfsWindowApi), alignof(MfsWindowApi), offsetof(MfsWindowApi, context), offsetof(MfsWindowApi, invoke),
        sizeof(MfsWindowResult), alignof(MfsWindowResult), offsetof(MfsWindowResult, first),
        offsetof(MfsWindowResult, second), offsetof(MfsWindowResult, scale), 6
    };
    if (!output || capacity < layout.size()) { return 1; }
    std::copy(layout.begin(), layout.end(), output);
    return 0;
}
