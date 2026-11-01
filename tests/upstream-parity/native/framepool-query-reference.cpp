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
#include <chrono>
#include <cstring>
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
thread_local void* zero_owner = nullptr;
thread_local bool zero_pending = false;
thread_local std::array<int64_t, 6> zero_request {};
thread_local std::array<int64_t, 31> zero_geometry {};
thread_local RECT queried_client {};
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
    if (zero_pending) {
        zero_pending = false;
        if (!zero_owner || mfs_print_thread_geometry(zero_owner, zero_request.data(), zero_request.size(),
                zero_geometry.data(), zero_geometry.size()) != 0
            || zero_geometry[14] != zero_request[4] || zero_geometry[15] != zero_request[5]
            || zero_geometry[10] <= zero_geometry[8] || zero_geometry[11] <= zero_geometry[9]) {
            throw std::runtime_error("actual zero-client UI resize failed");
        }
    }
    const BOOL result = GetClientRect(mode == 1 ? invalid() : window, output);
    const DWORD error = GetLastError();
    if (!result) { trace.win32 = error; }
    else if (mode == 5) { output->right = shift(output->left, 128); }
    else if (mode == 6) { output->bottom = shift(output->top, 96); }
    if (result) { queried_client = *output; }
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
#if defined(MFS_FRAMEPOOL_CONTENT_OBSERVER)
namespace {
void mfs_observe_frame_content(winrt::Windows::Graphics::SizeInt32 size)
{
    std::cout << "observed actual frame ContentSize=" << size.Width << ',' << size.Height << '\n';
}
}
#endif
#undef MAA_CTRL_UNIT_NS_BEGIN
#undef MAA_CTRL_UNIT_NS_END
#define MAA_CTRL_UNIT_NS_BEGIN namespace MfsFrameQueryOriginal {
#define MAA_CTRL_UNIT_NS_END }
#if defined(MFS_FRAMEPOOL_SIZE_OBSERVER)
// Every dependency of this header is already included above. Access changes
// only for this separately compiled test class; its fields/layout stay intact.
#define private public
#endif
#include "Screencap/FramePoolScreencap.h"
#if defined(MFS_FRAMEPOOL_SIZE_OBSERVER)
#undef private
#include "framepool-size-probe.hpp"
#endif
#if defined(MFS_FRAMEPOOL_MAP_OBSERVER)
#include "framepool-map-probe.hpp"
#define d3d_context_ ::mfs_map_probe::context(d3d_context_)
#define cached_image_ ::mfs_map_probe::cache(cached_image_)
#define bgra_to_bgr ::mfs_map_probe::convert
#endif
#define GetClientRect client
#define ClientToScreen origin
#define DwmGetWindowAttribute bounds
#define GetWindowRect fallback
#include MFS_UPSTREAM_FRAMEPOOL_CPP
#undef GetClientRect
#undef ClientToScreen
#undef DwmGetWindowAttribute
#undef GetWindowRect
#if defined(MFS_FRAMEPOOL_MAP_OBSERVER)
#undef d3d_context_
#undef cached_image_
#undef bgra_to_bgr
#endif

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
void save_raw(const std::filesystem::path& root, const std::string& name, const std::optional<cv::Mat>& image)
{
    std::ofstream layout(root / (name + ".txt"), std::ios::binary);
    layout.exceptions(std::ios::badbit | std::ios::failbit);
    std::ofstream pixels(root / (name + ".bgr"), std::ios::binary);
    pixels.exceptions(std::ios::badbit | std::ios::failbit);
    if (!image) { layout << "0\t0\t0\t0\n"; return; }
    layout << image->cols << '\t' << image->rows << '\t' << image->step[0] << "\t18432\n";
    if (image->type() != CV_8UC3 || image->cols != 96 || image->rows != 64) { throw std::runtime_error("source saved image layout differs"); }
    for (int y = 0; y < 64; ++y) { pixels.write(reinterpret_cast<const char*>(image->ptr(y)), 288); }
    pixels.flush(); layout.flush();
}
void save(const std::filesystem::path& root, const std::string& name, const std::optional<cv::Mat>& image)
{
    save_raw(root, name, image);
    if (image) { known(*image); }
}
void save_pair(const std::filesystem::path& root, const std::string& name,
    const std::optional<cv::Mat>& image, const std::optional<cv::Mat>& cache)
{
    // Retain both owned Mat observations even if the first known-color check fails.
    save_raw(root, name, image); save_raw(root, name + "-cache", cache);
    if (image) { known(*image); }
    if (cache) { known(*cache); }
}
int record_control(const std::filesystem::path& root)
{
    if (!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root)) { return 1; }
    try {
        auto image = [](uint32_t phase, uint32_t corrupt) {
            cv::Mat value(64, 96, CV_8UC3);
            for (int y = 0; y < 64; ++y) {
                for (int x = 0; x < 96; ++x) {
                    value.at<cv::Vec3b>(y, x) = x < 16 && y < 16
                        ? corrupt == 1 ? cv::Vec3b(0x1f, 0x53, 0xa7)
                            : phase == 0 ? cv::Vec3b(0x4e, 0x5f, 0x60) : cv::Vec3b(0x1a, 0x2b, 0x3c)
                        : x < 32 ? cv::Vec3b(0x1f, 0x53, 0xa7)
                        : y < 32 ? cv::Vec3b(0xb9, 0x4d, 0x27) : cv::Vec3b(0x3b, 0xe2, 0x81);
                }
            }
            if (corrupt == 2) { value.at<cv::Vec3b>(63, 95)[2] ^= 1; }
            return value;
        };
        std::ofstream rows(root / "record-control.tsv", std::ios::binary);
        rows.exceptions(std::ios::badbit | std::ios::failbit);
        rows << "case\trejected\timage\tcache\n";
        for (uint32_t index = 0; index < 8; ++index) {
            const uint32_t phase = index == 1;
            std::optional<cv::Mat> output, cache;
            if (index < 6) { output = image(phase, index == 2 ? 1 : index == 4 ? 2 : 0); }
            if (index < 7) { cache = image(phase, index == 3 ? 1 : index == 5 ? 2 : 0); }
            bool rejected = false;
            try { save_pair(root, "control-" + std::to_string(index), output, cache); }
            catch (const std::runtime_error& error) {
                const std::string_view message(error.what());
                if (message != "query source marker differs" && message != "query source full pixels differ") { throw; }
                rejected = true;
            }
            if (rejected != (index >= 2 && index <= 5)) { throw std::runtime_error("record control rejection differs"); }
            rows << index << '\t' << rejected << '\t' << output.has_value() << '\t' << cache.has_value() << '\n';
            rows.flush();
        }
        std::cout << "original FramePool CPU record controls cases=8 rejected=4 pixel_files=16 captures=0 passed\n";
        return 0;
    }
    catch (const std::exception& error) { std::cerr << "record control failure=" << error.what() << '\n'; }
    return 1;
}
int empty_client(const std::filesystem::path& root, bool prepare_restore = false)
{
    if (!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root)) { return 1; }
    try {
        Initialization initialization;
        winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED)); initialization.owned = true;
        std::ofstream rows(root / "empty.tsv", std::ios::binary), geometry(root / "geometry.tsv", std::ios::binary);
        rows.exceptions(std::ios::badbit | std::ios::failbit); geometry.exceptions(std::ios::badbit | std::ios::failbit);
        rows << "cycle\tcase\tphase\toutcome\torder\tclient_width\tclient_height\tmarker\tcaller_thread\tfixture_thread\n";
        geometry << "cycle\tphase\tclient_width\tclient_height\touter_width\touter_height\tstyle\towner_timer_errors\n";
        std::ofstream presentations;
        if (prepare_restore) {
            presentations.open(root / "restore-presentations.tsv", std::ios::binary);
            presentations.exceptions(std::ios::badbit | std::ios::failbit);
            presentations << "cycle\tbefore\tafter\towner_timer_errors\n";
        }
        struct Held { std::string name; std::optional<cv::Mat> image; };
        std::vector<Held> held; held.reserve(60);
        const auto write = [&](const std::string& name, const std::optional<cv::Mat>& image) {
            if (!image) { throw std::runtime_error("empty-client expected an actual source result"); }
            if (image->empty()) {
                if (image->cols || image->rows || image->step[0] || image->data || image->type()) { throw std::runtime_error("source empty Mat header differs"); }
                save(root, name, std::nullopt);
            }
            else { save(root, name, image); }
        };
        for (uint32_t cycle = 0; cycle < 20; ++cycle) {
            const uint32_t mode = cycle % 3;
            const int64_t width = mode == 1 ? 96 : 0, height = mode == 0 ? 64 : 0;
            Fixture fixture;
            if (mfs_print_thread_fixture(96, 64, 0, 0, 0, 0, MFS_RENDER_DISABLE_TRANSITIONS | MFS_RENDER_TOPMOST, &fixture.value) != 0) { throw std::runtime_error("empty fixture creation failed"); }
            std::array<int64_t, 6> restore { 1, WS_POPUP | WS_VISIBLE, 100, 100, 96, 64 };
            std::array<int64_t, 31> snapshot {};
            if (mfs_print_thread_geometry(fixture.value.owner, restore.data(), restore.size(), snapshot.data(), snapshot.size()) != 0
                || snapshot[14] != 96 || snapshot[15] != 64
                || mfs_desktop_fixture_inside_updates(fixture.value.owner) != 0) { throw std::runtime_error("empty warm fixture failed"); }
            std::array<uint64_t, 4> owner {};
            if (mfs_print_thread_info(fixture.value.owner, owner.data(), owner.size()) != 0 || !owner[0] || owner[0] != owner[2] || owner[0] == owner[1]) { throw std::runtime_error("empty fixture owner differs"); }
            {
                MfsFrameQueryOriginal::FramePoolScreencap source(static_cast<HWND>(fixture.value.hwnd));
                for (const auto phase : { "warm", "zero", "recovery" }) {
                    const bool zero = std::string_view(phase) == "zero";
                    if (std::string_view(phase) == "recovery"
                        && mfs_print_thread_geometry(fixture.value.owner, restore.data(), restore.size(), snapshot.data(), snapshot.size()) != 0) { throw std::runtime_error("empty restore failed"); }
                    if (std::string_view(phase) == "recovery" && prepare_restore) {
                        // Fixture readiness only: establish the whole restored
                        // scene and await two owner presentations before the
                        // one original capture. Immediate results stay separate.
                        std::array<int64_t, 6> redraw { 2, 0, 0, 0, 0, 0 };
                        std::array<uint64_t, 3> update {};
                        if (mfs_print_thread_geometry(fixture.value.owner, redraw.data(), redraw.size(), snapshot.data(), snapshot.size()) != 0
                            || mfs_desktop_fixture_update_info(fixture.value.owner, update.data(), update.size()) != 0
                            || !update[1] || update[1] > UINT64_MAX-2 || update[2]) { throw std::runtime_error("restore presentation setup failed"); }
                        const auto before = update[1];
                        const auto deadline = std::chrono::steady_clock::now()+std::chrono::seconds(2);
                        while (update[1] < before+2) {
                            if (std::chrono::steady_clock::now() >= deadline) { throw std::runtime_error("restore presentation deadline"); }
                            std::this_thread::sleep_for(std::chrono::milliseconds(5));
                            if (mfs_desktop_fixture_update_info(fixture.value.owner, update.data(), update.size()) != 0 || update[2]) { throw std::runtime_error("restore presentation failed"); }
                        }
                        presentations << cycle << '\t' << before << '\t' << update[1] << "\t0\n"; presentations.flush();
                    }
                    zero_owner = fixture.value.owner; zero_request = { 3, WS_POPUP | WS_VISIBLE | WS_BORDER, 100, 100, width, height };
                    zero_pending = zero; trace = { true, 0, 0, 0, 0 };
                    struct Stop { ~Stop() { trace.active = false; zero_pending = false; zero_owner = nullptr; } } stop;
#if defined(MFS_FRAMEPOOL_CONTENT_OBSERVER)
                    std::cout << "frame observer cycle=" << cycle << " phase=" << phase << std::endl;
#endif
                    auto image = source.screencap(); // No capture retry; intervention runs only after the actual Map.
                    trace.active = false;
                    if (trace.order != 123 || trace.win32 || trace.dwm || !image || image->empty() != zero
                        || queried_client.right != (zero ? width : 96) || queried_client.bottom != (zero ? height : 64)) { throw std::runtime_error("source zero-client result/query differs"); }
                    const auto name = std::to_string(cycle) + "-" + phase;
                    if (!zero) {
                        const auto pixel = image->at<cv::Vec3b>(0, 0);
                        HDC dc = GetDC(static_cast<HWND>(fixture.value.hwnd));
                        const auto corner = dc ? GetPixel(dc, 8, 8) : CLR_INVALID;
                        const auto body = dc ? GetPixel(dc, 40, 40) : CLR_INVALID;
                        if (dc && ReleaseDC(static_cast<HWND>(fixture.value.hwnd), dc) != 1) { throw std::runtime_error("diagnostic DC release failed"); }
                        std::cout << "empty observation cycle=" << cycle << " phase=" << phase << " bgr0="
                            << static_cast<unsigned>(pixel[0]) << ',' << static_cast<unsigned>(pixel[1]) << ',' << static_cast<unsigned>(pixel[2])
                            << " current_gdi_corner=" << corner << " current_gdi_body=" << body << '\n';
                    }
                    write(name, image);
                    const int marker = zero ? -1 : known(*image);
                    std::array<uint64_t, 3> updates {};
                    if (mfs_desktop_fixture_update_info(fixture.value.owner, updates.data(), updates.size()) != 0 || !updates[0] || !updates[1] || updates[2]) { throw std::runtime_error("empty-client timer/presentation failed"); }
                    if (zero) { snapshot = zero_geometry; }
                    else {
                        std::array<int64_t, 6> observe {};
                        if (mfs_print_thread_geometry(fixture.value.owner, observe.data(), observe.size(), snapshot.data(), snapshot.size()) != 0) { throw std::runtime_error("empty geometry observation failed"); }
                    }
                    rows << cycle << '\t' << mode << '\t' << phase << '\t' << (zero ? 2 : 1) << "\t123\t" << queried_client.right << '\t' << queried_client.bottom
                        << '\t' << marker << '\t' << owner[1] << '\t' << owner[0] << '\n';
                    geometry << cycle << '\t' << phase << '\t' << snapshot[14] << '\t' << snapshot[15] << '\t' << snapshot[10]-snapshot[8] << '\t' << snapshot[11]-snapshot[9] << '\t' << snapshot[18] << "\t0\n";
                    rows.flush(); geometry.flush(); held.push_back({name, std::move(image)});
                }
            }
            fixture.close();
        }
        RoUninitialize(); initialization.owned = false;
        for (const auto& entry : held) { write(entry.name + "-held", entry.image); }
        std::cout << "original FramePool empty_client cycles=20 calls=60 real_zero=20 recovery=20 retained=40"
            << (prepare_restore ? " restore_barriers=20" : "") << " passed\n";
        return 0;
    }
    catch (const winrt::hresult_error& error) { std::cerr << "empty HRESULT=" << error.code().value << '\n'; }
    catch (const std::exception& error) { std::cerr << "empty failure=" << error.what() << '\n'; }
    return 1;
}
}
#if defined(MFS_FRAMEPOOL_MAP_OBSERVER)
#include "framepool-map-cases.hpp"
#endif
#if defined(MFS_FRAMEPOOL_SIZE_OBSERVER)
#include "framepool-size-cases.hpp"
#endif
int wmain(int argc, wchar_t** argv)
{
    if (argc == 3 && std::wstring_view(argv[1]) == L"--record-control") { return record_control(argv[2]); }
#if defined(MFS_FRAMEPOOL_SIZE_OBSERVER)
    if (argc == 3 && std::wstring_view(argv[1]) == L"--size-boundary") { return size_boundary(argv[2]); }
#endif
#if defined(MFS_FRAMEPOOL_MAP_OBSERVER)
    if (argc == 3 && std::wstring_view(argv[1]) == L"--map-boundary") { return map_boundary(argv[2]); }
#endif
    if (argc == 3 && std::wstring_view(argv[1]) == L"--empty-client") { return empty_client(argv[2]); }
    if (argc == 3 && std::wstring_view(argv[1]) == L"--empty-client-prepared") { return empty_client(argv[2], true); }
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
                    const auto name = std::to_string(cycle) + "-" + phase;
                    save_raw(root, name, image);
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
