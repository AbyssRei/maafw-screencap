// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Standalone observation of the unmodified fixed FramePool implementation.
// Each caller context runs in a new process: no cached factory or implicit MTA
// from a previous mode can establish the next mode's initial environment.
#include "Screencap/FramePoolScreencap.h"
#include "Screencap/FramePoolWithPseudoMinimizeScreencap.h"
#include "print-thread.h"
#include "print-pseudo.h"
#include <dispatcherqueue.h>
#include <roapi.h>
#include <winrt/Windows.System.h>
#include <winrt/Windows.Foundation.h>
#include <filesystem>
#include <array>
#include <fstream>
#include <iostream>
#include <limits>
#include <stdexcept>
#include <string_view>
#include <thread>

namespace {
using WindowState = std::array<uint64_t, 6>;
WindowState window_state(void* hwnd)
{
    WindowState state {};
    if (mfs_print_pseudo_state(hwnd, state.data(), state.size()) != 0) {
        throw std::runtime_error("fixture window state query failed");
    }
    return state;
}
void save_states(const std::filesystem::path& directory, const WindowState& before,
    const WindowState& captured, const WindowState& inactive, const WindowState& closed)
{
    std::ofstream file(directory / "window-states.txt");
    file.exceptions(std::ios::badbit | std::ios::failbit);
    const auto row = [&](const char* name, const WindowState& values) {
        file << name;
        for (const auto value : values) { file << ' ' << value; }
        file << '\n';
    };
    row("before", before);
    row("captured", captured);
    row("inactive", inactive);
    row("closed", closed);
}
void apartment(const char* point)
{
    APTTYPE type = APTTYPE_CURRENT;
    APTTYPEQUALIFIER qualifier = APTTYPEQUALIFIER_NONE;
    const HRESULT code = CoGetApartmentType(&type, &qualifier);
    std::cout << "apartment " << point << " hr=" << code << " type=" << type
        << " qualifier=" << qualifier << " thread=" << GetCurrentThreadId() << std::endl;
}

struct Initialization {
    bool owned = false;
    ~Initialization() { if (owned) { RoUninitialize(); } }
};

struct Dispatcher {
    winrt::Windows::System::DispatcherQueueController controller { nullptr };
    winrt::Windows::Foundation::IAsyncAction shutdown { nullptr };
    bool close()
    {
        if (!controller) { return true; }
        if (!shutdown) { shutdown = controller.ShutdownQueueAsync(); }
        // Completion, not a sleep budget, establishes destruction permission.
        // This is harness shutdown only; source capture never pumps/retries.
        const auto began = std::chrono::steady_clock::now();
        bool over_budget = false;
        while (shutdown.Status() == winrt::Windows::Foundation::AsyncStatus::Started) {
            MSG message {};
            while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) {
                if (message.message == WM_QUIT) { throw std::runtime_error("unexpected WM_QUIT"); }
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            over_budget |= std::chrono::steady_clock::now() - began > std::chrono::seconds(5);
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
        shutdown.GetResults();
        shutdown = nullptr;
        controller = nullptr;
        std::cout << "dispatcher_shutdown=completed over_budget=" << over_budget << std::endl;
        return !over_budget;
    }
    ~Dispatcher()
    {
        try { if (!close()) { std::cerr << "dispatcher shutdown exceeded observation budget\n"; } }
        catch (...) { std::cerr << "dispatcher cleanup failed\n"; std::terminate(); }
    }
};

struct Fixture {
    MfsPrintFixture value {};
    bool close_failed = false;
    bool close()
    {
        if (!value.owner) { return true; }
        if (close_failed) { return false; }
        const auto status = mfs_print_thread_close(&value.owner);
        close_failed = status != 0;
        std::cout << "fixture_close=" << status << std::endl;
        return status == 0;
    }
    ~Fixture()
    {
        // A failed owner close is never retried or freed while callbacks exist.
        // This isolated process must fail instead of unloading their code.
        if (value.owner && !close()) { std::terminate(); }
    }
};

bool save(const cv::Mat& image, const std::filesystem::path& directory)
{
    if (image.empty() || image.type() != CV_8UC3 || image.cols <= 0 || image.rows <= 0) {
        throw std::runtime_error("unexpected source image layout");
    }
    const auto width = static_cast<size_t>(image.cols);
    if (width > std::numeric_limits<size_t>::max() / 3) { throw std::runtime_error("source row overflow"); }
    const auto row = width * 3;
    if (row > static_cast<size_t>(std::numeric_limits<std::streamsize>::max())) {
        throw std::runtime_error("source row exceeds stream width");
    }
    std::ofstream bytes(directory / "source.bgr", std::ios::binary);
    bytes.exceptions(std::ios::badbit | std::ios::failbit);
    for (int y = 0; y < image.rows; ++y) {
        bytes.write(reinterpret_cast<const char*>(image.ptr(y)), static_cast<std::streamsize>(row));
    }
    std::cout << "source_image width=" << image.cols << " height=" << image.rows
        << " stride=" << image.step[0] << " exported_row=" << row << std::endl;
    // This records a known interior independently of source success. It is not
    // a full image/format/resize assertion or evidence of Rust parity.
    bool interior = image.cols > 16 && image.rows > 16;
    for (int y = 8; y < image.rows - 8; ++y) {
        for (int x = 8; x < image.cols - 8; ++x) {
            const cv::Vec3b expected = x < image.cols / 3 ? cv::Vec3b(0x1f, 0x53, 0xa7)
                : y < image.rows / 2 ? cv::Vec3b(0xb9, 0x4d, 0x27) : cv::Vec3b(0x3b, 0xe2, 0x81);
            interior &= image.at<cv::Vec3b>(y, x) == expected;
        }
    }
    std::cout << "known_interior_pixels=" << interior << std::endl;
    return interior;
}

uint64_t digest(const cv::Mat& image)
{
    uint64_t hash = 0xcbf29ce484222325ULL;
    for (int y = 0; y < image.rows; ++y) {
        for (int x = 0; x < image.cols; ++x) {
            for (int c = 0; c < 3; ++c) {
                // Unsigned wrap is the explicitly defined FNV-1a operation.
                hash = (hash ^ image.at<cv::Vec3b>(y, x)[c]) * 0x100000001b3ULL;
            }
        }
    }
    return hash;
}
}

int wmain(int argc, wchar_t** argv)
{
    if (argc != 3 && argc != 4) {
        std::cerr << "usage: mfs_framepool_reference <uninitialized|mta|mta-dispatcher|sta> <existing-evidence-dir> [plain|pseudo-visible|pseudo-stop|pseudo-inactive|pseudo-layered-stop|pseudo-layered-inactive|control-zero|control-zero-repaint|control-opaque|control-opaque-repaint]\n";
        return 1;
    }
    const std::wstring_view mode(argv[1]);
    const std::filesystem::path output(argv[2]);
    const std::wstring_view scenario = argc == 4 ? argv[3] : L"plain";
    if ((mode != L"uninitialized" && mode != L"mta" && mode != L"mta-dispatcher" && mode != L"sta")
        || (scenario != L"plain" && scenario != L"pseudo-visible" && scenario != L"pseudo-stop"
            && scenario != L"pseudo-inactive" && scenario != L"pseudo-layered-stop" && scenario != L"pseudo-layered-inactive"
            && scenario != L"control-zero" && scenario != L"control-zero-repaint"
            && scenario != L"control-opaque" && scenario != L"control-opaque-repaint")
        || !output.is_absolute() || !std::filesystem::is_directory(output)) { return 1; }
    const bool pseudo = scenario.starts_with(L"pseudo-");
    const bool control = scenario.starts_with(L"control-");
    const bool minimized = control || (pseudo && scenario != L"pseudo-visible");
    const bool layered = scenario.find(L"layered") != std::wstring_view::npos;
    const bool call_inactive = scenario.find(L"inactive") != std::wstring_view::npos;
    std::wcout << L"source_only_context=" << mode << std::endl;
    std::wcout << L"source_scenario=" << scenario << std::endl;
    apartment("initial");
    Initialization initialization;
    try {
        if (mode != L"uninitialized") {
            const auto code = RoInitialize(mode == L"sta" ? RO_INIT_SINGLETHREADED : RO_INIT_MULTITHREADED);
            std::cout << "explicit_RoInitialize=" << code << std::endl;
            winrt::check_hresult(code);
            initialization.owned = true;
        }
        Dispatcher dispatcher;
        if (mode == L"mta-dispatcher") {
            const DispatcherQueueOptions options { sizeof(DispatcherQueueOptions), DQTYPE_THREAD_CURRENT, DQTAT_COM_NONE };
            winrt::check_hresult(CreateDispatcherQueueController(options,
                reinterpret_cast<PDISPATCHERQUEUECONTROLLER*>(winrt::put_abi(dispatcher.controller))));
        }
        apartment("before_source");
        Fixture fixture;
        const auto created = mfs_print_thread_fixture(96, 64, 0, 0, 0, 0,
            MFS_RENDER_DISABLE_TRANSITIONS | MFS_RENDER_TOPMOST, &fixture.value);
        std::cout << "fixture_create=" << created << std::endl;
        if (created != 0) { return 2; }
        uint64_t owner[4] {};
        if (mfs_print_thread_info(fixture.value.owner, owner, 4) != 0) { return 2; }
        std::cout << "fixture_owner=" << owner[0] << " caller=" << owner[1]
            << " native_owner=" << owner[2] << std::endl;
        if (minimized) {
            const auto prepared = mfs_print_thread_prepare(fixture.value.owner, layered ? 1u : 0u);
            std::cout << "fixture_minimize_prepare=" << prepared << std::endl;
            if (prepared != 0) { return prepared == 5 ? 77 : 2; }
        }
        if (control) {
            const auto alpha = scenario.find(L"opaque") != std::wstring_view::npos ? 255u : 0u;
            const auto repaint = scenario.ends_with(L"repaint") ? 1u : 0u;
            const auto restored = mfs_print_thread_restore_control(fixture.value.owner, alpha, repaint);
            std::cout << "fixture_control_restore=" << restored << " alpha=" << alpha << " repaint=" << repaint << std::endl;
            if (restored != 0) { return 2; }
            uint64_t statistics[13] {};
            if (mfs_print_thread_stats(fixture.value.owner, statistics, 13) != 0) { return 2; }
            std::cout << "fixture_control_stats";
            for (const auto value : statistics) { std::cout << ' ' << value; }
            std::cout << std::endl;
        }
        const auto before = window_state(fixture.value.hwnd);
        WindowState captured {}, inactive {};
        cv::Mat retained;
        bool capture_failed = false;
        bool pixels_match = false;
        uint64_t retained_digest = 0;
        const auto observe = [&](auto& source) {
            const auto began = std::chrono::steady_clock::now();
            std::optional<cv::Mat> result;
            try {
                result = source.screencap(); // Exactly one call. No retry or pumping inside capture.
                std::cout << "source_result=" << (result ? "image" : "empty") << std::endl;
                capture_failed = !result;
            }
            catch (const winrt::hresult_error& error) {
                capture_failed = true;
                std::cout << "source_result=hresult_exception hr=" << error.code().value << std::endl;
            }
            catch (const std::exception& error) {
                capture_failed = true;
                std::cout << "source_result=exception message=" << error.what() << std::endl;
            }
            std::cout << "source_elapsed_ms=" << std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::steady_clock::now() - began).count() << std::endl;
            apartment("after_capture");
            // Export failures belong to the harness, outside source exception
            // classification and elapsed capture time. Preserve the raw result.
            if (result) {
                retained = std::move(*result);
                pixels_match = save(retained, output);
                retained_digest = digest(retained);
            }
            captured = window_state(fixture.value.hwnd);
            if (call_inactive) { source.inactive(); }
            inactive = window_state(fixture.value.hwnd);
        };
        if (pseudo) {
            MaaNS::CtrlUnitNs::FramePoolWithPseudoMinimizeScreencap source(static_cast<HWND>(fixture.value.hwnd));
            observe(source);
        }
        else {
            MaaNS::CtrlUnitNs::FramePoolScreencap source(static_cast<HWND>(fixture.value.hwnd));
            observe(source);
        }
        apartment("after_source_destructor");
        save_states(output, before, captured, inactive, window_state(fixture.value.hwnd));
        if (!fixture.close()) { return 3; }
        std::cout << "retained_after_source_and_window_close=" << !retained.empty() << std::endl;
        if (!retained.empty() && digest(retained) != retained_digest) {
            throw std::runtime_error("retained source pixels changed after source/window close");
        }
        if (!dispatcher.close()) { return 3; }
        apartment("before_owned_uninitialize");
        // Initialization's successful reference is balanced on every exit path.
        // Observe what remains without trying to undo the source's implicit MTA.
        if (initialization.owned) { RoUninitialize(); initialization.owned = false; }
        apartment("final");
        return capture_failed ? 4 : pixels_match ? 0 : 5;
    }
    catch (const winrt::hresult_error& error) {
        std::cout << "harness_hresult=" << error.code().value << std::endl;
        return 3;
    }
    catch (const std::exception& error) {
        std::cout << "harness_error=" << error.what() << std::endl;
        return 3;
    }
}
