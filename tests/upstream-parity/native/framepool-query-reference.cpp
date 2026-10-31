// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// All SDK declarations precede interposition. Only this private original class
// uses the four query wrappers; the renderer and OS calls remain independent.
#include "Base/UnitBase.h"
#include "Common/Conf.h"
#include "Screencap/SafeDXGI.hpp"
#include "Screencap/HwndUtils.hpp"
#include "MaaUtils/Logger.h"
#include "MaaUtils/NoWarningCV.hpp"
#include "MaaUtils/Time.hpp"
#include "print-thread.h"
#include <dwmapi.h>
#include <roapi.h>
#include <windows.graphics.capture.interop.h>
#include <windows.graphics.directx.direct3d11.interop.h>
#include <winrt/Windows.Graphics.Capture.h>
#include <winrt/Windows.Foundation.Metadata.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Security.Authorization.AppCapabilityAccess.h>
#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <limits>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

namespace {
struct Trace { bool active = false; uint32_t mode = 0, order = 0, win32 = 0; int32_t dwm = 0; };
thread_local Trace trace;
uint32_t query(uint32_t operation)
{
    if (!trace.active) { return UINT32_MAX; }
    if (trace.order > 999) { throw std::runtime_error("extra source query"); }
    trace.order = trace.order * 10 + operation;
    return trace.mode;
}
LONG shift(LONG value, LONG offset)
{
    const auto result = static_cast<int64_t>(value) + offset;
    if (result < LONG_MIN || result > LONG_MAX) { throw std::runtime_error("query fixture coordinate overflow"); }
    return static_cast<LONG>(result);
}
HWND invalid() { return reinterpret_cast<HWND>(static_cast<uintptr_t>(1)); }
BOOL client(HWND window, RECT* output)
{
    const auto mode = query(1);
    const BOOL result = GetClientRect(mode == 1 ? invalid() : window, output);
    const DWORD error = GetLastError();
    if (!result) { trace.win32 = error; }
    else if (mode == 5) { output->right = shift(output->left, 128); }
    else if (mode == 6) { output->bottom = shift(output->top, 96); }
    SetLastError(error);
    return result;
}
BOOL origin(HWND window, POINT* point)
{
    const auto mode = query(2);
    const BOOL result = ClientToScreen(mode == 2 ? invalid() : window, point);
    const DWORD error = GetLastError();
    if (!result) { trace.win32 = error; }
    else if (mode == 7) { point->x = shift(point->x, 128); }
    else if (mode == 8) { point->y = shift(point->y, 96); }
    else if (mode == 9) { point->x = shift(point->x, -8); point->y = shift(point->y, -8); }
    SetLastError(error);
    return result;
}
HRESULT bounds(HWND window, DWORD attribute, void* output, DWORD size)
{
    const auto mode = query(3);
    const HRESULT result = DwmGetWindowAttribute(mode == 3 || mode == 4 ? invalid() : window, attribute, output, size);
    const DWORD error = GetLastError();
    if (FAILED(result)) { trace.dwm = static_cast<int32_t>(result); }
    SetLastError(error);
    return result;
}
BOOL fallback(HWND window, RECT* output)
{
    const auto mode = query(4);
    const BOOL result = GetWindowRect(mode == 4 ? invalid() : window, output);
    const DWORD error = GetLastError();
    if (!result) { trace.win32 = error; }
    SetLastError(error);
    return result;
}
}
namespace MfsFrameQueryOriginal {
using MAA_CTRL_UNIT_NS::ScreencapBase;
using MAA_CTRL_UNIT_NS::bgra_to_bgr;
using MaaNS::duration_since;
}
#undef MAA_CTRL_UNIT_NS_BEGIN
#undef MAA_CTRL_UNIT_NS_END
#define MAA_CTRL_UNIT_NS_BEGIN namespace MfsFrameQueryOriginal {
#define MAA_CTRL_UNIT_NS_END }
#define GetClientRect client
#define ClientToScreen origin
#define DwmGetWindowAttribute bounds
#define GetWindowRect fallback
#include MFS_UPSTREAM_FRAMEPOOL_CPP
#undef GetClientRect
#undef ClientToScreen
#undef DwmGetWindowAttribute
#undef GetWindowRect

namespace {
struct Initialization { bool owned = false; ~Initialization() { if (owned) { RoUninitialize(); } } };
struct Fixture {
    MfsPrintFixture value {};
    bool failed = false;
    void close()
    {
        if (failed) { throw std::runtime_error("fixture close previously failed"); }
        if (value.owner && mfs_print_thread_close(&value.owner) != 0) {
            failed = true; throw std::runtime_error("fixture close/join failed");
        }
    }
    ~Fixture() { if (value.owner) { try { close(); } catch (...) { std::terminate(); } } }
};
int known(const cv::Mat& image)
{
    if (image.type() != CV_8UC3 || image.cols != 96 || image.rows != 64) { throw std::runtime_error("query source image layout differs"); }
    const auto marker = image.at<cv::Vec3b>(0, 0);
    const int phase = marker == cv::Vec3b(0x4e, 0x5f, 0x60) ? 0 : marker == cv::Vec3b(0x1a, 0x2b, 0x3c) ? 1 : -1;
    if (phase < 0) { throw std::runtime_error("query source marker differs"); }
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 96; ++x) {
            const auto expected = x < 16 && y < 16 ? marker : x < 32 ? cv::Vec3b(0x1f, 0x53, 0xa7)
                : y < 32 ? cv::Vec3b(0xb9, 0x4d, 0x27) : cv::Vec3b(0x3b, 0xe2, 0x81);
            if (image.at<cv::Vec3b>(y, x) != expected) { throw std::runtime_error("query source full pixels differ"); }
        }
    }
    return phase;
}
void save(const std::filesystem::path& root, const std::string& name, const std::optional<cv::Mat>& image)
{
    std::ofstream layout(root / (name + ".txt"), std::ios::binary);
    layout.exceptions(std::ios::badbit | std::ios::failbit);
    std::ofstream pixels(root / (name + ".bgr"), std::ios::binary);
    pixels.exceptions(std::ios::badbit | std::ios::failbit);
    if (!image) { layout << "0\t0\t0\t0\n"; return; }
    known(*image);
    layout << "96\t64\t" << image->step[0] << "\t18432\n";
    for (int y = 0; y < 64; ++y) { pixels.write(reinterpret_cast<const char*>(image->ptr(y)), 288); }
}
}
int wmain(int argc, wchar_t** argv)
{
    if (argc != 2) { return 1; }
    const std::filesystem::path root(argv[1]);
    if (!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root)) { return 1; }
    try {
        Initialization initialization;
        winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED)); initialization.owned = true;
        std::ofstream rows(root / "queries.tsv", std::ios::binary);
        rows.exceptions(std::ios::badbit | std::ios::failbit);
        rows << "cycle\tcase\tphase\tstatus\torder\twin32\tdwm\tmarker\tcaller_thread\tfixture_thread\tticks\n";
        struct Held { std::string name; std::optional<cv::Mat> image; };
        std::vector<Held> held; held.reserve(60);
        for (uint32_t cycle = 0; cycle < 20; ++cycle) {
            const uint32_t mode = cycle % 10;
            Fixture fixture;
            if (mfs_print_thread_fixture(96, 64, 0, 0, 0, 0, MFS_RENDER_DISABLE_TRANSITIONS | MFS_RENDER_TOPMOST, &fixture.value) != 0) {
                throw std::runtime_error("query fixture creation failed");
            }
            std::array<int64_t, 6> input { 1, WS_POPUP | WS_VISIBLE, 100, 100, 96, 64 };
            std::array<int64_t, 31> geometry {};
            if (mfs_print_thread_geometry(fixture.value.owner, input.data(), input.size(), geometry.data(), geometry.size()) != 0
                || geometry[14] != 96 || geometry[15] != 64) { throw std::runtime_error("query fixture geometry premise failed"); }
            if (mfs_desktop_fixture_inside_updates(fixture.value.owner) != 0) { throw std::runtime_error("query marker start failed"); }
            std::array<uint64_t, 4> owner {};
            if (mfs_print_thread_info(fixture.value.owner, owner.data(), owner.size()) != 0
                || !owner[0] || owner[0] != owner[2] || owner[0] == owner[1]) { throw std::runtime_error("query fixture owner differs"); }
            {
                MfsFrameQueryOriginal::FramePoolScreencap source(static_cast<HWND>(fixture.value.hwnd));
                for (const auto phase : { "warm", "probe", "recovery" }) {
                    const bool probe = std::string_view(phase) == "probe";
                    const uint32_t current = probe ? mode : 0;
                    trace = { true, current, 0, 0, 0 };
                    struct Stop { ~Stop() { trace.active = false; } } stop;
                    auto image = source.screencap(); // No capture retry.
                    trace.active = false;
                    const auto observed = trace;
                    const bool failure = probe && (mode == 1 || mode == 2 || mode == 4);
                    const uint32_t expected_order = current == 1 ? 1 : current == 2 ? 12 : current == 3 || current == 4 ? 1234 : 123;
                    if (observed.order != expected_order || image.has_value() == failure
                        || (failure && !observed.win32) || ((current == 3 || current == 4) && observed.dwm >= 0)) {
                        throw std::runtime_error("source query branch/fault was not reached");
                    }
                    const int marker = image ? known(*image) : -1;
                    std::array<uint64_t, 3> updates {};
                    if (mfs_desktop_fixture_update_info(fixture.value.owner, updates.data(), updates.size()) != 0
                        || !updates[0] || !updates[1] || updates[2]) { throw std::runtime_error("query marker presentation failed"); }
                    const auto name = std::to_string(cycle) + "-" + phase;
                    save(root, name, image);
                    rows << cycle << '\t' << mode << '\t' << phase << '\t' << (image ? 1 : 0) << '\t' << observed.order
                        << '\t' << observed.win32 << '\t' << observed.dwm << '\t' << marker << '\t' << owner[1] << '\t' << owner[0] << '\t' << updates[1] << '\n';
                    rows.flush(); held.push_back({ name, std::move(image) });
                }
            }
            fixture.close();
        }
        RoUninitialize(); initialization.owned = false;
        for (const auto& entry : held) { save(root, entry.name + "-held", entry.image); }
        std::cout << "original FramePool queries cycles=20 cases=10 calls=60 images=54 rejected=6 recovery=20 retained=54 passed\n";
        return 0;
    }
    catch (const winrt::hresult_error& error) { std::cerr << "query HRESULT=" << error.code().value << '\n'; }
    catch (const std::exception& error) { std::cerr << "query failure=" << error.what() << '\n'; }
    return 1;
}
