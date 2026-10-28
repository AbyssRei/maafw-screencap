// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "print-pseudo.h"
#include "gdi-probe-internal.hpp"
#include "Screencap/PrintWindowWithPseudoMinimizeScreencap.h"
#include <memory>
using Original = MAA_CTRL_UNIT_NS::PrintWindowWithPseudoMinimizeScreencap;
using OriginalHelper = MAA_CTRL_UNIT_NS::PseudoMinimizeHelper;

int32_t mfs_print_pseudo_open(void* hwnd, void** owner)
{
    if (!owner) { return 1; }
    *owner = nullptr;
    try { *owner = new Original(static_cast<HWND>(hwnd)); return 0; }
    catch (const std::bad_alloc&) { return 2; }
    catch (...) { return 3; }
}
int32_t mfs_print_pseudo_capture(void* owner, MfsParityImage* image)
{
    if (!image) { return 1; }
    *image = {};
    if (!owner) { return 1; }
    try {
        auto pixels = static_cast<Original*>(owner)->screencap();
        if (!pixels) { return 4; }
        return mfs_parity::transfer_image(std::move(*pixels), image);
    }
    catch (const std::bad_alloc&) { return 2; }
    catch (...) { return 3; }
}
int32_t mfs_print_pseudo_inactive(void* owner)
{
    if (!owner) { return 1; }
    try { static_cast<Original*>(owner)->inactive(); return 0; }
    catch (...) { return 3; }
}
int32_t mfs_print_pseudo_close(void** owner)
{
    if (!owner) { return 1; }
    // The upstream destructor synchronously stops/joins the monitor. No retry
    // or timeout: completion precedes owner invalidation and module release.
    delete static_cast<Original*>(*owner);
    *owner = nullptr;
    return 0;
}
int32_t mfs_print_pseudo_prepare(void* target, void* other, uint32_t layered)
{
    HWND hwnd = static_cast<HWND>(target);
    HWND anchor = static_cast<HWND>(other);
    if (!IsWindow(hwnd) || !IsWindow(anchor) || hwnd == anchor || layered > 1) { return 1; }
    SetLastError(0);
    LONG_PTR style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if (!style && GetLastError()) { return 2; }
    style = layered ? style | WS_EX_LAYERED : style & ~static_cast<LONG_PTR>(WS_EX_LAYERED);
    SetLastError(0);
    if (!SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style) && GetLastError()) { return 2; }
    if (layered && !SetLayeredWindowAttributes(hwnd, 0, 73, LWA_ALPHA)) { return 2; }
    ShowWindow(hwnd, SW_MINIMIZE);
    SetActiveWindow(anchor);
    SetForegroundWindow(anchor);
    if (!IsIconic(hwnd) || GetForegroundWindow() == hwnd) { return 5; }
    return 0;
}
int32_t mfs_print_pseudo_state(void* target, uint64_t* state, size_t capacity)
{
    if (!state || capacity < 6) { return 1; }
    HWND hwnd = static_cast<HWND>(target);
    if (!IsWindow(hwnd)) { return 1; }
    SetLastError(0);
    const LONG_PTR style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if (!style && GetLastError()) { return 2; }
    BYTE alpha = 255;
    COLORREF color = 0;
    DWORD flags = 0;
    if ((style & WS_EX_LAYERED) && !GetLayeredWindowAttributes(hwnd, &color, &alpha, &flags)) { return 2; }
    RECT client {};
    if (!GetClientRect(hwnd, &client)) { return 2; }
    state[0] = static_cast<uint64_t>(style);
    state[1] = alpha;
    state[2] = IsIconic(hwnd) != FALSE;
    state[3] = GetForegroundWindow() == hwnd;
    state[4] = static_cast<uint64_t>(client.right - client.left);
    state[5] = static_cast<uint64_t>(client.bottom - client.top);
    return 0;
}

int32_t mfs_pseudo_helper_open(void* hwnd, void** owner)
{
    if (!owner) { return 1; }
    *owner = nullptr;
    try {
        auto helper = std::make_unique<OriginalHelper>(static_cast<HWND>(hwnd));
        helper->start();
        *owner = helper.release();
        return 0;
    }
    catch (const std::bad_alloc&) { return 2; }
    catch (...) { return 3; }
}
int32_t mfs_pseudo_helper_ensure(void* owner)
{
    if (!owner) { return 1; }
    try { static_cast<OriginalHelper*>(owner)->ensure_not_minimized(); return 0; }
    catch (...) { return 3; }
}
int32_t mfs_pseudo_helper_status(void* owner, uint32_t* pseudo)
{
    if (!pseudo) { return 1; }
    *pseudo = 0;
    if (!owner) { return 1; }
    *pseudo = static_cast<OriginalHelper*>(owner)->is_pseudo_minimized() ? 1u : 0u;
    return 0;
}
int32_t mfs_pseudo_helper_close(void** owner)
{
    if (!owner) { return 1; }
    delete static_cast<OriginalHelper*>(*owner);
    *owner = nullptr;
    return 0;
}
