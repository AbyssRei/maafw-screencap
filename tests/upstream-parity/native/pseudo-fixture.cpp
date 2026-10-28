// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Shared acceptance window controls; no screenshot backend or source helper.
#include "print-pseudo.h"
#include <windows.h>

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
