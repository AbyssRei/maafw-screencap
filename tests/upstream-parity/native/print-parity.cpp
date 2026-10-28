// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "print-parity.h"
#include "gdi-probe-internal.hpp"
#if defined(MFS_UPSTREAM_PRINT_CPP)
#include "Screencap/PrintWindowScreencap.h"
#endif
#include <array>
#include <algorithm>
#include <climits>
#include <cstdio>
#include <limits>
#include <new>
#include <dwmapi.h>

namespace {
struct Pattern {
    bool popup = false;
    uint64_t paints = 0;
    uint64_t prints = 0;
    bool failed = false;
    uintptr_t paint_wparam = 0;
    uintptr_t paint_dc = 0;
    uintptr_t paint_bitmap = 0;
};
struct Fixture {
    HINSTANCE module = nullptr;
    wchar_t name[64] {};
    bool registered = false;
    HWND main = nullptr;
    HWND child = nullptr;
    HWND popup = nullptr;
    Pattern main_pattern {};
    Pattern child_pattern {};
    Pattern popup_pattern { true };
    bool transitions_disabled = false;
};

void draw(HWND hwnd, HDC dc, Pattern& pattern) noexcept
{
    RECT client {};
    if (!GetClientRect(hwnd, &client)) { pattern.failed = true; return; }
    const std::array<RECT, 3> regions {
        RECT { 0, 0, client.right / 3, client.bottom },
        RECT { client.right / 3, 0, client.right, client.bottom / 2 },
        RECT { client.right / 3, client.bottom / 2, client.right, client.bottom }
    };
    const std::array<COLORREF, 3> colors = pattern.popup
        ? std::array<COLORREF, 3> { 0x00e34219, 0x00578bc1, 0x009d23f5 }
        : std::array<COLORREF, 3> { 0x001f53a7, 0x00b94d27, 0x003be281 };
    for (size_t i = 0; i < regions.size(); ++i) {
        if (regions[i].right <= regions[i].left || regions[i].bottom <= regions[i].top) { continue; }
        HBRUSH brush = CreateSolidBrush(colors[i]);
        if (!brush) { pattern.failed = true; return; }
        const int filled = FillRect(dc, &regions[i], brush);
        const BOOL deleted = DeleteObject(brush);
        if (!filled || !deleted) { pattern.failed = true; return; }
    }
}

LRESULT CALLBACK window_proc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) noexcept
{
    if (message == WM_NCCREATE) {
        const auto* create = reinterpret_cast<const CREATESTRUCTW*>(lparam);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(create->lpCreateParams));
    }
    auto* pattern = reinterpret_cast<Pattern*>(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    if (pattern && message == WM_PAINT) {
        PAINTSTRUCT paint {};
        HDC dc = BeginPaint(hwnd, &paint);
        pattern->paint_wparam = static_cast<uintptr_t>(wparam);
        pattern->paint_dc = reinterpret_cast<uintptr_t>(dc);
        pattern->paint_bitmap = reinterpret_cast<uintptr_t>(GetCurrentObject(dc, OBJ_BITMAP));
        if (dc) { draw(hwnd, dc, *pattern); } else { pattern->failed = true; }
        if (!EndPaint(hwnd, &paint)) { pattern->failed = true; }
        if (pattern->paints == std::numeric_limits<uint64_t>::max()) { pattern->failed = true; }
        else { ++pattern->paints; }
        return 0;
    }
    if (pattern && (message == WM_PRINTCLIENT || message == WM_PRINT)) {
        draw(hwnd, reinterpret_cast<HDC>(wparam), *pattern);
        if (pattern->prints == std::numeric_limits<uint64_t>::max()) { pattern->failed = true; }
        else { ++pattern->prints; }
        return 0;
    }
    if (message == WM_NCDESTROY) { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0); }
    return DefWindowProcW(hwnd, message, wparam, lparam);
}

#if defined(MFS_UPSTREAM_PRINT_CPP)
bool region(int32_t x, int32_t y, int32_t width, int32_t height, const cv::Mat& image)
{
    return x >= 0 && y >= 0 && width > 0 && height > 0
        && static_cast<int64_t>(x) + width <= image.cols
        && static_cast<int64_t>(y) + height <= image.rows;
}
#endif
}

#if defined(MFS_UPSTREAM_PRINT_CPP)
int32_t mfs_print_parity_reference(void* hwnd, MfsParityImage* output)
{
    if (!output) { return 1; }
    *output = {};
    try {
        MAA_CTRL_UNIT_NS::PrintWindowScreencap capture(static_cast<HWND>(hwnd));
        auto pixels = capture.screencap();
        return pixels ? mfs_parity::transfer_image(std::move(*pixels), output) : 1;
    }
    catch (const std::bad_alloc&) { return 3; }
    catch (...) { return 2; }
}

int32_t mfs_print_parity_overlay(void* destination, void* source, int32_t sx, int32_t sy,
    int32_t dx, int32_t dy, int32_t width, int32_t height)
{
    if (!destination || !source || destination == source) { return 1; }
    try {
        auto& target = mfs_parity::image_pixels(destination);
        const auto& input = mfs_parity::image_pixels(source);
        if (!region(sx, sy, width, height, input) || !region(dx, dy, width, height, target)) { return 1; }
        input(cv::Rect(sx, sy, width, height)).copyTo(target(cv::Rect(dx, dy, width, height)));
        return 0;
    }
    catch (const std::bad_alloc&) { return 3; }
    catch (...) { return 2; }
}

#endif

static int32_t create_fixture(int32_t width, int32_t height, int32_t pw, int32_t ph,
    int32_t offset_x, int32_t offset_y, uint32_t mode, bool child_target, MfsPrintFixture* output)
{
    if (!output) { return 1; }
    *output = {};
    const bool disable_transitions = (mode & MFS_RENDER_DISABLE_TRANSITIONS) != 0;
    mode &= ~MFS_RENDER_DISABLE_TRANSITIONS;
    if (disable_transitions && (mode != 0 || child_target)) { return 1; }
    const int64_t px = 260LL + offset_x + (child_target ? 7 : 0);
    const int64_t py = 180LL + offset_y + (child_target ? 5 : 0);
    const int64_t root_width = static_cast<int64_t>(width) + (child_target ? 24 : 0);
    const int64_t root_height = static_cast<int64_t>(height) + (child_target ? 18 : 0);
    if (width <= 0 || height <= 0 || pw < 0 || ph < 0 || mode > 2
        || px < INT_MIN || px > INT_MAX || py < INT_MIN || py > INT_MAX
        || root_width > INT_MAX || root_height > INT_MAX) { return 1; }
    auto* fixture = new (std::nothrow) Fixture;
    if (!fixture) { return 3; }
    output->owner = fixture; // All subsequent failures retain explicit cleanup ownership.
    if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        reinterpret_cast<LPCWSTR>(&window_proc), &fixture->module)) { return 1; }
    if (swprintf_s(fixture->name, L"mfs-print-fixture-%p", static_cast<void*>(fixture)) < 0) { return 1; }
    WNDCLASSEXW window_class {};
    window_class.cbSize = sizeof(window_class);
    window_class.lpfnWndProc = window_proc;
    window_class.hInstance = fixture->module;
    window_class.lpszClassName = fixture->name;
    if (!RegisterClassExW(&window_class)) { return 1; }
    fixture->registered = true;
    fixture->main = CreateWindowExW(0, fixture->name, L"mfs PrintWindow main", WS_POPUP | (disable_transitions ? 0 : WS_VISIBLE),
        260, 180, static_cast<int>(root_width), static_cast<int>(root_height), nullptr, nullptr,
        fixture->module, &fixture->main_pattern);
    if (!fixture->main) { return 1; }
    output->hwnd = fixture->main;
    if (disable_transitions) {
        // Configure this owned window while hidden, before its first show.
        // No global animation setting, capture sleep or pixel retry is added.
        const BOOL value = TRUE;
        if (FAILED(DwmSetWindowAttribute(fixture->main, DWMWA_TRANSITIONS_FORCEDISABLED, &value, sizeof(value)))) { return 1; }
        fixture->transitions_disabled = true;
        ShowWindow(fixture->main, SW_SHOWNORMAL);
        if (!IsWindowVisible(fixture->main)) { return 1; }
    }
    RECT client {};
    if (!GetClientRect(fixture->main, &client) || client.right != root_width || client.bottom != root_height) { return 1; }
    UpdateWindow(fixture->main);
    if (child_target) {
        fixture->child = CreateWindowExW(0, fixture->name, L"mfs PrintWindow child", WS_CHILD | WS_VISIBLE,
            7, 5, width, height, fixture->main, nullptr, fixture->module, &fixture->child_pattern);
        if (!fixture->child) { return 1; }
        output->hwnd = fixture->child;
        if (!GetClientRect(fixture->child, &client) || client.right != width || client.bottom != height) { return 1; }
        UpdateWindow(fixture->child);
    }
    output->width = client.right;
    output->height = client.bottom;
    if (mode != 0) {
        fixture->popup = CreateWindowExW(0, fixture->name, L"mfs PrintWindow popup", WS_POPUP | WS_VISIBLE,
            static_cast<int>(px), static_cast<int>(py), pw, ph, fixture->main, nullptr,
            fixture->module, &fixture->popup_pattern);
        if (!fixture->popup) { return 1; }
        output->popup = fixture->popup;
        if (!GetClientRect(fixture->popup, &client) || client.right != pw || client.bottom != ph) { return 1; }
        output->popup_width = client.right;
        output->popup_height = client.bottom;
        UpdateWindow(fixture->popup);
        SetActiveWindow(fixture->popup);
        if (GetLastActivePopup(fixture->main) != fixture->popup) { return 1; }
        if (mode == 2 && (!SetWindowPos(fixture->popup, nullptr, 0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_HIDEWINDOW)
            || IsWindowVisible(fixture->popup))) { return 1; }
    }
    // PrintWindow's RENDERFULLCONTENT path consumes the rendered window. The
    // fixture must finish initial GDI/compositor presentation before either side
    // captures, including a main window with no activating owned popup.
    if (!GdiFlush() || FAILED(DwmFlush()) || fixture->main_pattern.failed
        || fixture->child_pattern.failed || fixture->popup_pattern.failed) { return 1; }
    return 0;
}

int32_t mfs_print_parity_fixture(int32_t width, int32_t height, int32_t pw, int32_t ph,
    int32_t offset_x, int32_t offset_y, uint32_t mode, MfsPrintFixture* output)
{
    return create_fixture(width, height, pw, ph, offset_x, offset_y, mode, false, output);
}
int32_t mfs_print_parity_child_fixture(int32_t width, int32_t height, int32_t pw, int32_t ph,
    int32_t offset_x, int32_t offset_y, uint32_t mode, MfsPrintFixture* output)
{
    return create_fixture(width, height, pw, ph, offset_x, offset_y, mode, true, output);
}

int32_t mfs_print_parity_close(void** owner)
{
    if (!owner) { return 1; }
    auto* fixture = static_cast<Fixture*>(*owner);
    if (!fixture) { return 0; }
    if (fixture->popup) {
        if (!DestroyWindow(fixture->popup)) { return 1; }
        fixture->popup = nullptr;
    }
    if (fixture->child) {
        if (!DestroyWindow(fixture->child)) { return 1; }
        fixture->child = nullptr;
    }
    if (fixture->main) {
        if (!DestroyWindow(fixture->main)) { return 1; }
        fixture->main = nullptr;
    }
    if (fixture->registered) {
        if (!UnregisterClassW(fixture->name, fixture->module)) { return 1; }
        fixture->registered = false;
    }
    delete fixture;
    *owner = nullptr;
    return 0;
}

int32_t mfs_print_parity_stats(void* owner, uint64_t* output, size_t capacity)
{
    if (!owner || !output || capacity < 5) { return 1; }
    const auto& fixture = *static_cast<Fixture*>(owner);
    const auto& target = fixture.child ? fixture.child_pattern : fixture.main_pattern;
    output[0] = target.paints;
    output[1] = target.prints;
    output[2] = fixture.popup_pattern.paints;
    output[3] = fixture.popup_pattern.prints;
    output[4] = fixture.main_pattern.failed || fixture.child_pattern.failed || fixture.popup_pattern.failed;
    if (capacity >= 11) {
        output[5] = target.paint_wparam;
        output[6] = target.paint_dc;
        output[7] = target.paint_bitmap;
        output[8] = fixture.popup_pattern.paint_wparam;
        output[9] = fixture.popup_pattern.paint_dc;
        output[10] = fixture.popup_pattern.paint_bitmap;
    }
    if (capacity >= 12) { output[11] = fixture.transitions_disabled; }
    return 0;
}

int32_t mfs_print_parity_layout(uint64_t* output, size_t capacity)
{
    constexpr std::array<uint64_t, 9> layout {
        sizeof(MfsPrintFixture), alignof(MfsPrintFixture), offsetof(MfsPrintFixture, owner),
        offsetof(MfsPrintFixture, hwnd), offsetof(MfsPrintFixture, popup), offsetof(MfsPrintFixture, width),
        offsetof(MfsPrintFixture, height), offsetof(MfsPrintFixture, popup_width), offsetof(MfsPrintFixture, popup_height)
    };
    if (!output || capacity < layout.size()) { return 1; }
    std::copy(layout.begin(), layout.end(), output);
    if (capacity >= 10) { output[9] = MFS_RENDER_DISABLE_TRANSITIONS; }
    return 0;
}

#if defined(MFS_UPSTREAM_PRINT_CPP)
int32_t mfs_print_parity_intersection(int32_t cw, int32_t ch, int32_t pw, int32_t ph,
    int64_t ox, int64_t oy, int32_t* output, size_t capacity)
{
    if (!output || capacity < 6) { return 2; }
    std::fill_n(output, 6, 0);
    if (cw <= 0 || ch <= 0 || pw <= 0 || ph <= 0
        || ox < INT_MIN || ox > INT_MAX || oy < INT_MIN || oy > INT_MAX) { return 2; }
    const int offset_x = static_cast<int>(ox);
    const int offset_y = static_cast<int>(oy);
    const cv::Rect canvas(0, 0, cw, ch);
    const cv::Rect popup(offset_x, offset_y, pw, ph);
    const cv::Rect destination = canvas & popup;
    if (destination.empty()) { return 1; }
    // OpenCV's intersection proves these source differences fit the popup.
    output[0] = destination.x - offset_x;
    output[1] = destination.y - offset_y;
    output[2] = destination.x;
    output[3] = destination.y;
    output[4] = destination.width;
    output[5] = destination.height;
    return 0;
}
#endif
