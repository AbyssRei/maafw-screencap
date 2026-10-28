// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Include unedited upstream helper in an isolated namespace. Explicit fake
// scheduling permits deterministic monitor iterations; real lifetime tests are
// separate and must not be inferred from this script.
#include "pseudo-script.h"
#include "MaaUtils/SafeWindows.hpp"
#include "MaaUtils/Logger.h"
#include "Common/Conf.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <functional>
#include <thread>

namespace {
struct Script {
    uint32_t scenario = 0;
    LONG_PTR style = 0;
    BYTE alpha = 255;
    bool valid = true;
    bool iconic = true;
    bool foreground = false;
    uint32_t reads = 0, writes = 0, alpha_writes = 0;
    MfsPseudoReport report {};
    std::function<void()> monitor;
};
thread_local Script script;
void event(uint32_t operation, int64_t argument, int64_t result, uint32_t error = 0)
{
    if (script.report.count == 64) { script.report.overflow = 1; return; }
    script.report.events[script.report.count++] = { operation, error, argument, result };
}
DWORD failure(bool fail)
{
    const DWORD error = fail ? 0xe0120000u | script.scenario : 0;
    SetLastError(error);
    return error;
}
LONG_PTR get_style(HWND, int)
{
    ++script.reads;
    const DWORD error = failure((script.scenario == 4 && script.reads == 1)
        || (script.scenario == 6 && script.reads == 2));
    const LONG_PTR value = error ? 0 : script.style;
    event(1, 0, value, error);
    return value;
}
LONG_PTR set_style(HWND, int, LONG_PTR value)
{
    ++script.writes;
    const DWORD error = failure((script.scenario == 7 && script.writes == 1)
        || ((script.scenario == 9 || script.scenario == 20 || script.scenario == 21) && script.writes == 2));
    const LONG_PTR previous = error ? 0 : script.style;
    event(2, value, previous, error);
    if (!error) { script.style = value; }
    return previous;
}
BOOL get_alpha(HWND, COLORREF* color, BYTE* alpha, DWORD* flags)
{
    const DWORD error = failure(script.scenario == 5 || script.scenario == 21);
    if (!error) { *color = 0; *alpha = script.alpha; *flags = LWA_ALPHA; }
    event(3, 0, error ? 0 : script.alpha, error);
    return error ? FALSE : TRUE;
}
BOOL set_alpha(HWND, COLORREF color, BYTE alpha, DWORD flags)
{
    if (color != 0 || flags != LWA_ALPHA) { script.report.overflow = 1; }
    ++script.alpha_writes;
    const DWORD error = failure((script.scenario == 8 && script.alpha_writes == 1)
        || ((script.scenario == 10 || script.scenario == 20 || script.scenario == 21) && script.alpha_writes == 2));
    event(4, alpha, error ? 0 : 1, error);
    if (!error) { script.alpha = alpha; }
    return error ? FALSE : TRUE;
}
BOOL valid(HWND) { event(5, 0, script.valid); return script.valid; }
BOOL iconic(HWND) { event(6, 0, script.iconic); return script.iconic; }
HWND foreground()
{
    event(7, 0, script.foreground);
    return script.foreground ? reinterpret_cast<HWND>(1) : nullptr;
}
BOOL show(HWND, int command)
{
    const bool minimize = command == SW_MINIMIZE;
    if (!minimize && command != SW_SHOWNOACTIVATE) { script.report.overflow = 1; }
    const bool result = script.scenario != 11;
    event(8, minimize, result);
    script.iconic = minimize;
    return result;
}
}

// All other std names used by upstream/logger remain the real standard library.
// Only this included helper's thread/sleep are scripted. No definitions are
// added to namespace std, and the pristine native helper is a separate TU.
namespace MfsPseudoScriptStd {
using namespace std;
class thread {
    bool joinable_ = false;
public:
    thread() = default;
    template<class F, class T> thread(F function, T* object) : joinable_(true)
    {
        script.monitor = [function, object] { (object->*function)(); };
    }
    thread& operator=(thread&& other) noexcept
    {
        joinable_ = other.joinable_;
        other.joinable_ = false;
        return *this;
    }
    bool joinable() const { return joinable_; }
    void join() { joinable_ = false; script.monitor = {}; }
};
namespace this_thread {
void sleep_for(std::chrono::milliseconds duration)
{
    event(9, duration.count(), 0);
    script.valid = false; // explicitly terminate the next monitor iteration
}
}
}

#undef MAA_CTRL_UNIT_NS_BEGIN
#undef MAA_CTRL_UNIT_NS_END
#define MAA_CTRL_UNIT_NS_BEGIN namespace MfsPseudoScriptReference {
#define MAA_CTRL_UNIT_NS_END }
#undef GetWindowLongPtr
#undef SetWindowLongPtr
#define std MfsPseudoScriptStd
#define GetWindowLongPtr get_style
#define SetWindowLongPtr set_style
#define GetLayeredWindowAttributes get_alpha
#define SetLayeredWindowAttributes set_alpha
#define IsWindow valid
#define IsIconic iconic
#define GetForegroundWindow foreground
#define ShowWindow show
#include MFS_UPSTREAM_PSEUDO_CPP
#undef std
#undef GetWindowLongPtr
#undef SetWindowLongPtr
#undef GetLayeredWindowAttributes
#undef SetLayeredWindowAttributes
#undef IsWindow
#undef IsIconic
#undef GetForegroundWindow
#undef ShowWindow

int32_t mfs_pseudo_script(uint32_t scenario, MfsPseudoReport* output)
{
    if (!output) { return 1; }
    *output = {};
    if (scenario > 21) { return 1; }
    try {
        script = {};
        script.scenario = scenario;
        script.style = scenario == 0 ? 0 : 0x40000;
        if (scenario == 1 || scenario == 5 || scenario == 10 || scenario == 20 || scenario == 21) {
            script.style |= WS_EX_LAYERED;
            script.alpha = 73;
        }
        script.valid = scenario != 3;
        script.iconic = scenario != 2 && scenario != 15;
        MfsPseudoScriptReference::PseudoMinimizeHelper helper(reinterpret_cast<HWND>(1));
        auto snapshot = [&](size_t index) {
            const std::array<uint64_t, 5> values {
                static_cast<uint64_t>(script.style), script.alpha, helper.is_pseudo_minimized(),
                script.iconic, script.valid
            };
            std::copy(values.begin(), values.end(), script.report.snapshots[index]);
        };
        if (scenario == 17) { helper.revert_pseudo_minimize(); }
        helper.start();
        if (scenario == 16) { helper.start(); }
        snapshot(0);
        helper.ensure_not_minimized();
        snapshot(1);
        if (scenario == 12 || scenario == 20) {
            if (helper.is_pseudo_minimized()) { helper.revert_pseudo_minimize(); }
        }
        if (scenario == 13 || scenario == 14 || scenario == 15 || scenario == 19) {
            script.valid = scenario != 14;
            script.foreground = scenario == 13;
            script.monitor();
            helper.start(); // monitor exit does not clear U's running flag
        }
        if (scenario == 16) { helper.ensure_not_minimized(); helper.start(); }
        if (scenario == 18) { script.iconic = true; helper.ensure_not_minimized(); }
        snapshot(2);
        helper.stop();
        if (scenario == 16) { helper.stop(); }
        snapshot(3);
        *output = script.report;
        return output->overflow ? 2 : 0;
    }
    catch (...) { script.monitor = {}; return 3; }
}

int32_t mfs_pseudo_script_layout(uint64_t* output, size_t capacity)
{
    constexpr std::array<uint64_t, 11> layout {
        sizeof(MfsPseudoEvent), alignof(MfsPseudoEvent), offsetof(MfsPseudoEvent, error),
        offsetof(MfsPseudoEvent, argument), offsetof(MfsPseudoEvent, result),
        sizeof(MfsPseudoReport), alignof(MfsPseudoReport), offsetof(MfsPseudoReport, overflow),
        offsetof(MfsPseudoReport, events), offsetof(MfsPseudoReport, snapshots), 64
    };
    if (!output || capacity < layout.size()) { return 1; }
    std::copy(layout.begin(), layout.end(), output);
    return 0;
}
