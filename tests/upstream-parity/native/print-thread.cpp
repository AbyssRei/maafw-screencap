// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "print-thread.h"
#include "print-pseudo.h"
#include "MaaUtils/SafeWindows.hpp"
#include <dwmapi.h>
#include <algorithm>
#include <array>
#include <condition_variable>
#include <mutex>
#include <new>
#include <thread>

namespace {
constexpr UINT command_message = WM_APP + 0x71;
enum class Command { none, stats, close, activate, prepare, restore_control };
struct Owner {
    std::thread worker;
    std::mutex mutex;
    std::condition_variable changed;
    bool ready = false;
    bool exited = false;
    bool pending = false;
    bool replied = false;
    bool close_failed = false;
    int32_t create_status = 3;
    int32_t command_status = 3;
    int32_t exit_cleanup_status = 3;
    DWORD thread = 0;
    uint32_t parameter = 0;
    Command command = Command::none;
    MfsPrintFixture fixture {};
    HWND anchor = nullptr;
    std::array<uint64_t, 13> stats {};
    std::array<uint64_t, 5> activation {};
};
struct Arguments {
    int32_t width, height, popup_width, popup_height, x, y;
    uint32_t mode;
    bool child;
};

int32_t close_native(Owner& owner)
{
    int32_t status = mfs_print_parity_close(&owner.fixture.owner);
    if (owner.anchor) {
        if (DestroyWindow(owner.anchor)) { owner.anchor = nullptr; }
        else if (status == 0) { status = 1; }
    }
    return status;
}

void run(Owner& owner, Arguments arguments) noexcept
{
    bool ready = false;
    try {
        // PostThreadMessage requires an existing queue. Publish the id only
        // after queue creation and native window setup have completed.
        MSG queued {};
        PeekMessageW(&queued, nullptr, 0, 0, PM_NOREMOVE);
        const DWORD id = GetCurrentThreadId();
        const auto create = arguments.child ? mfs_print_parity_child_fixture : mfs_print_parity_fixture;
        int32_t status = create(arguments.width, arguments.height, arguments.popup_width,
            arguments.popup_height, arguments.x, arguments.y, arguments.mode, &owner.fixture);
        // Creation/activation can queue additional paint/input messages. Finish
        // that owned setup queue before publishing the initial fixture state.
        // A bounded drain is a setup assertion, never a capture retry/wait.
        size_t drained = 0;
        while (status == 0 && PeekMessageW(&queued, nullptr, 0, 0, PM_REMOVE)) {
            if (++drained > 4096 || queued.message == WM_QUIT) { status = 7; break; }
            TranslateMessage(&queued);
            DispatchMessageW(&queued);
        }
        if (status == 0 && !GdiFlush()) { status = 7; }
        {
            std::lock_guard lock(owner.mutex);
            owner.thread = id;
            owner.create_status = status;
            owner.ready = true;
            ready = true;
        }
        owner.changed.notify_all();
        MSG message {};
        bool closed = false;
        int received = 0;
        while (!closed && (received = GetMessageW(&message, nullptr, 0, 0)) > 0) {
            if (!message.hwnd && message.message == command_message) {
                Command command;
                uint32_t parameter;
                {
                    std::lock_guard lock(owner.mutex);
                    if (!owner.pending) { continue; }
                    command = owner.command;
                    parameter = owner.parameter;
                    owner.pending = false;
                }
                int32_t result = 1;
                if (command == Command::stats) {
                    // Pattern counters belong to this UI thread. Never inspect
                    // them concurrently with a callback on the capture thread.
                    result = mfs_print_parity_stats(owner.fixture.owner, owner.stats.data(), owner.stats.size());
                }
                else if (command == Command::close) {
                    result = close_native(owner);
                    closed = result == 0;
                }
                else if (command == Command::activate) {
                    SetActiveWindow(static_cast<HWND>(owner.fixture.hwnd));
                    const BOOL accepted = SetForegroundWindow(static_cast<HWND>(owner.fixture.hwnd));
                    const HWND foreground = GetForegroundWindow();
                    DWORD process = 0;
                    GetWindowThreadProcessId(foreground, &process);
                    owner.activation = { static_cast<uint64_t>(accepted),
                        reinterpret_cast<uintptr_t>(GetActiveWindow()), reinterpret_cast<uintptr_t>(foreground),
                        reinterpret_cast<uintptr_t>(owner.fixture.hwnd), process };
                    result = foreground == owner.fixture.hwnd ? 0 : 5;
                }
                else if (command == Command::prepare) {
#if defined(MFS_UPSTREAM_PRINT_CPP) || defined(MFS_FRAMEPOOL_PSEUDO_FIXTURE)
                    if (!owner.anchor) {
                        owner.anchor = CreateWindowExW(WS_EX_TOOLWINDOW, L"STATIC", L"mfs activation anchor",
                            WS_POPUP | WS_VISIBLE, 700, 500, 37, 23, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
                        if (owner.anchor) { UpdateWindow(owner.anchor); }
                    }
                    result = owner.anchor ? mfs_print_pseudo_prepare(owner.fixture.hwnd, owner.anchor, parameter) : 1;
#else
                    result = 1; // ScreenDC reuses rendering/owner lifetime only.
#endif
                }
                else if (command == Command::restore_control) {
                    const HWND hwnd = static_cast<HWND>(owner.fixture.hwnd);
                    const BYTE alpha = static_cast<BYTE>(parameter & 255u);
                    SetLastError(0);
                    const LONG_PTR old = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    result = (!old && GetLastError()) ? 2 : 0;
                    if (result == 0) {
                        SetLastError(0);
                        if (!SetWindowLongPtrW(hwnd, GWL_EXSTYLE, old | WS_EX_LAYERED | WS_EX_TRANSPARENT) && GetLastError()) { result = 2; }
                    }
                    if (result == 0 && !SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA)) { result = 2; }
                    if (result == 0) {
                        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                        if (parameter & 256u) {
                            if (!RedrawWindow(hwnd, nullptr, nullptr, RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN)
                                || !GdiFlush() || FAILED(DwmFlush())) { result = 2; }
                        }
                    }
                }
                {
                    std::lock_guard lock(owner.mutex);
                    owner.command_status = result;
                    owner.replied = true;
                    if (command == Command::close && result != 0) { owner.close_failed = true; }
                }
                owner.changed.notify_all();
            }
            else {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        // Unexpected quit/error is distinct from the explicit close protocol.
        // Attempt its one owner-thread cleanup, preserving the result for close.
        const int32_t cleanup = closed ? 0 : close_native(owner);
        {
            std::lock_guard lock(owner.mutex);
            owner.exit_cleanup_status = cleanup;
            owner.exited = true;
        }
        owner.changed.notify_all();
    }
    catch (...) {
        // Every native resource/context remains owned even when the worker
        // cannot continue. A failed close retains this module and opaque owner.
        // No exception escapes a std::thread or private C boundary.
        std::lock_guard lock(owner.mutex);
        if (!ready) { owner.ready = true; owner.create_status = 3; }
        owner.exited = true;
        owner.exit_cleanup_status = 3;
        owner.changed.notify_all();
    }
}

int32_t request(Owner& owner, Command command, uint32_t parameter = 0)
{
    DWORD id = 0;
    {
        std::lock_guard lock(owner.mutex);
        if (owner.exited || owner.pending || owner.close_failed) { return 1; }
        owner.command = command;
        owner.parameter = parameter;
        owner.pending = true;
        owner.replied = false;
        id = owner.thread;
    }
    if (!PostThreadMessageW(id, command_message, 0, 0)) {
        std::lock_guard lock(owner.mutex);
        owner.pending = false;
        if (command == Command::close) { owner.close_failed = true; }
        return 2;
    }
    std::unique_lock lock(owner.mutex);
    owner.changed.wait(lock, [&] { return owner.replied || owner.exited; });
    return owner.replied ? owner.command_status : 1;
}

int32_t create(Arguments arguments, MfsPrintFixture* output)
{
    if (!output) { return 1; }
    *output = {};
    try {
        auto* owner = new (std::nothrow) Owner;
        if (!owner) { return 3; }
        output->owner = owner; // includes thread-spawn and partial setup failure
        owner->worker = std::thread(run, std::ref(*owner), arguments);
        std::unique_lock lock(owner->mutex);
        owner->changed.wait(lock, [&] { return owner->ready; });
        const auto native = owner->fixture;
        output->hwnd = native.hwnd;
        output->popup = native.popup;
        output->width = native.width;
        output->height = native.height;
        output->popup_width = native.popup_width;
        output->popup_height = native.popup_height;
        return owner->create_status;
    }
    catch (...) { return 3; }
}
}

int32_t mfs_print_thread_fixture(int32_t width, int32_t height, int32_t pw, int32_t ph,
    int32_t x, int32_t y, uint32_t mode, MfsPrintFixture* output)
{
    return create({ width, height, pw, ph, x, y, mode, false }, output);
}
int32_t mfs_print_thread_child_fixture(int32_t width, int32_t height, int32_t pw, int32_t ph,
    int32_t x, int32_t y, uint32_t mode, MfsPrintFixture* output)
{
    return create({ width, height, pw, ph, x, y, mode, true }, output);
}
int32_t mfs_print_thread_stats(void* context, uint64_t* output, size_t capacity)
{
    if (!context || !output || capacity < 5) { return 1; }
    try {
        auto& owner = *static_cast<Owner*>(context);
        const int32_t status = request(owner, Command::stats);
        if (status) { return status; }
        std::lock_guard lock(owner.mutex);
        std::copy_n(owner.stats.begin(), std::min(capacity, owner.stats.size()), output);
        return 0;
    }
    catch (...) { return 3; }
}
int32_t mfs_print_thread_activate(void* context)
{
    if (!context) { return 1; }
    try { return request(*static_cast<Owner*>(context), Command::activate); }
    catch (...) { return 3; }
}
int32_t mfs_print_thread_activation_info(void* context, uint64_t* output, size_t capacity)
{
    if (!context || !output || capacity < 5) { return 1; }
    try {
        auto& owner = *static_cast<Owner*>(context);
        std::lock_guard lock(owner.mutex);
        std::copy(owner.activation.begin(), owner.activation.end(), output);
        return 0;
    }
    catch (...) { return 3; }
}
int32_t mfs_print_thread_prepare(void* context, uint32_t layered)
{
    if (!context || layered > 1) { return 1; }
    try { return request(*static_cast<Owner*>(context), Command::prepare, layered); }
    catch (...) { return 3; }
}
int32_t mfs_print_thread_restore_control(void* context, uint32_t alpha, uint32_t repaint)
{
    if (!context || (alpha != 0 && alpha != 255) || repaint > 1) { return 1; }
    try { return request(*static_cast<Owner*>(context), Command::restore_control, alpha | (repaint << 8)); }
    catch (...) { return 3; }
}
int32_t mfs_print_thread_info(void* context, uint64_t* output, size_t capacity)
{
    if (!context || !output || capacity < 4) { return 1; }
    try {
        auto& owner = *static_cast<Owner*>(context);
        DWORD thread;
        HWND window;
        {
            std::lock_guard lock(owner.mutex);
            thread = owner.thread;
            window = static_cast<HWND>(owner.fixture.hwnd);
        }
        output[0] = thread;
        output[1] = GetCurrentThreadId();
        output[2] = GetWindowThreadProcessId(window, nullptr);
        output[3] = GetCurrentProcessId();
        return 0;
    }
    catch (...) { return 3; }
}
int32_t mfs_print_thread_close(void** context)
{
    if (!context) { return 1; }
    if (!*context) { return 0; }
    try {
        auto* owner = static_cast<Owner*>(*context);
        int32_t status = 0;
        if (owner->worker.joinable()) {
            bool exited;
            {
                std::lock_guard lock(owner->mutex);
                if (owner->close_failed) { return 1; }
                exited = owner->exited;
                if (exited) { status = owner->exit_cleanup_status; }
            }
            if (!exited) { status = request(*owner, Command::close); }
            if (status != 0) { return status; }
            owner->worker.join();
        }
        delete owner;
        *context = nullptr;
        return 0;
    }
    catch (...) { return 3; }
}
