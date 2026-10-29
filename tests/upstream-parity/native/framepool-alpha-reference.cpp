// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Source-only alpha/content diagnostic; no product capture changes.
// Reuse the existing independently pumped window, source exporter and RAII.
#define wmain mfs_original_context_entry
#include "framepool-reference.cpp"
#undef wmain
#include "Screencap/PrintWindowScreencap.h"
#include <algorithm>
#include <memory>
#include <vector>

bool matches_scene(const cv::Mat& image, bool alternative)
{
    if (image.cols != 96 || image.rows != 64 || image.type() != CV_8UC3) { return false; }
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 96; ++x) {
            const auto region = x < 32 ? 0 : y < 32 ? 1 : 2;
            const std::array<cv::Vec3b, 3> normal { cv::Vec3b(0x1f,0x53,0xa7), cv::Vec3b(0xb9,0x4d,0x27), cv::Vec3b(0x3b,0xe2,0x81) };
            const std::array<cv::Vec3b, 3> other { cv::Vec3b(0xe3,0x42,0x19), cv::Vec3b(0x57,0x8b,0xc1), cv::Vec3b(0x9d,0x23,0xf5) };
            if (image.at<cv::Vec3b>(y,x) != (alternative ? other : normal)[region]) { return false; }
        }
    }
    return true;
}

int wmain(int argc, wchar_t** argv)
{
    if (argc != 2 || !std::filesystem::is_directory(argv[1])) { return 64; }
    const std::filesystem::path root(argv[1]);
    try {
        Initialization initialization;
        winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED));
        initialization.owned = true;
        Dispatcher dispatcher;
        const DispatcherQueueOptions options { sizeof(DispatcherQueueOptions), DQTYPE_THREAD_CURRENT, DQTAT_COM_NONE };
        winrt::check_hresult(CreateDispatcherQueueController(options,
            reinterpret_cast<PDISPATCHERQUEUECONTROLLER*>(winrt::put_abi(dispatcher.controller))));
        std::vector<cv::Mat> held;
        std::vector<uint64_t> hashes;
        std::ofstream rows(root / "observations.tsv");
        rows.exceptions(std::ios::badbit | std::ios::failbit);
        rows << "arm\tphase\talpha\tframe_image\tframe_known\tframe_zero\tprint_image\tprint_known\tpaint_before\tpaint_after\tframe_prior\telapsed_us\n";
        bool controls = true;
        size_t closed_windows = 0;
        for (const bool persistent : {false, true}) {
            Fixture fixture;
            if (mfs_print_thread_fixture(96, 64, 0, 0, 0, 0,
                    MFS_RENDER_DISABLE_TRANSITIONS | MFS_RENDER_TOPMOST, &fixture.value) != 0) {
                throw std::runtime_error("fixture creation failed");
            }
            if (mfs_print_thread_prepare(fixture.value.owner, 0) != 0) {
                throw std::runtime_error("fixture minimize preparation failed");
            }
            std::unique_ptr<MaaNS::CtrlUnitNs::FramePoolScreencap> shared;
            const HWND hwnd = static_cast<HWND>(fixture.value.hwnd);
            unsigned phase = 0;
            for (const unsigned alpha : {255u, 0u, 255u, 0u, 255u}) {
                const bool alternative = alpha == 0;
                if (mfs_print_thread_scene(fixture.value.owner, alternative ? 1u : 0u) != 0) {
                    throw std::runtime_error("controlled owner scene update failed");
                }
                const auto directory = root / ((persistent ? "persistent-" : "fresh-") + std::to_string(phase));
                std::filesystem::create_directory(directory);
                uint64_t before[13] {}, after[13] {};
                if (mfs_print_thread_stats(fixture.value.owner, before, 13) != 0
                    || mfs_print_thread_restore_control(fixture.value.owner, alpha, 1) != 0
                    || mfs_print_thread_stats(fixture.value.owner, after, 13) != 0) {
                    throw std::runtime_error("owner redraw control failed");
                }
                COLORREF key = 0;
                BYTE observed_alpha = 17;
                DWORD flags = 0;
                if (!GetLayeredWindowAttributes(hwnd, &key, &observed_alpha, &flags)
                    || observed_alpha != alpha || flags != LWA_ALPHA || IsIconic(hwnd)) {
                    throw std::runtime_error("alpha/window precondition failed");
                }
                const auto before_state = window_state(hwnd);
                std::unique_ptr<MaaNS::CtrlUnitNs::FramePoolScreencap> fresh;
                if (persistent && !shared) { shared = std::make_unique<MaaNS::CtrlUnitNs::FramePoolScreencap>(hwnd); }
                if (!persistent) { fresh = std::make_unique<MaaNS::CtrlUnitNs::FramePoolScreencap>(hwnd); }
                const auto began = std::chrono::steady_clock::now();
                auto result = (persistent ? shared : fresh)->screencap(); // one distinct source call per phase
                const auto elapsed = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::steady_clock::now()-began).count();
                const auto after_state = window_state(hwnd);
                std::ofstream states(directory / "states.tsv");
                states.exceptions(std::ios::badbit | std::ios::failbit);
                const auto state_row = [&](const char* label, const WindowState& values) {
                    states << label;
                    for (const auto value : values) { states << '\t' << value; }
                    states << '\n';
                };
                state_row("before_capture", before_state);
                state_row("after_capture", after_state);
                states.close();
                bool known = false, zero = false;
                bool prior = false;
                if (result) {
                    if (result->cols != 96 || result->rows != 64) {
                        throw std::runtime_error("unexpected target dimensions");
                    }
                    // The shared exporter also logs its original default
                    // palette check. The current-scene assertion below is the
                    // authority for this diagnostic's alternate palette.
                    save(*result, directory);
                    known = matches_scene(*result, alternative);
                    prior = matches_scene(*result, !alternative);
                    zero = true;
                    for (int y = 0; y < result->rows; ++y) {
                        const auto* row = result->ptr<uint8_t>(y);
                        zero &= std::all_of(row, row + 96 * 3, [](uint8_t value) { return value == 0; });
                    }
                    hashes.push_back(digest(*result));
                    held.push_back(std::move(*result));
                }
                MaaNS::CtrlUnitNs::PrintWindowScreencap printer(hwnd);
                auto printed = printer.screencap(); // independent native content witness, not a FramePool retry
                const auto print_directory = directory / "print";
                std::filesystem::create_directory(print_directory);
                if (printed) { save(*printed, print_directory); }
                const bool print_known = printed && matches_scene(*printed, alternative);
                rows << persistent << '\t' << phase << '\t' << alpha << '\t' << bool(result) << '\t'
                     << known << '\t' << zero << '\t' << bool(printed) << '\t' << print_known
                     << '\t' << before[0] << '\t' << after[0] << '\t' << prior << '\t' << elapsed << '\n';
                rows.flush();
                std::cout << "arm=" << persistent << " phase=" << phase << " alpha=" << alpha
                          << " image=" << bool(result) << " known=" << known << " zero=" << zero
                          << " print_known=" << print_known << " prior=" << prior << " elapsed_us=" << elapsed << std::endl;
                controls &= alpha == 0 || known;
                controls &= print_known;
                ++phase;
            }
            shared.reset();

            if (!fixture.close()) { return 3; }
            ++closed_windows;
        }
        for (size_t index = 0; index < held.size(); ++index) {
            if (digest(held[index]) != hashes[index]) { throw std::runtime_error("held source image mutated"); }
        }
        std::cout << "held_after_both_window_and_source_close=" << held.size() << std::endl;
        if (!dispatcher.close()) { return 3; }
        for (size_t index = 0; index < held.size(); ++index) {
            if (digest(held[index]) != hashes[index]) { throw std::runtime_error("held image changed during dispatcher shutdown"); }
        }
        std::cout << "positive_controls=" << controls << std::endl;
        std::ofstream summary(root / "summary.tsv");
        summary.exceptions(std::ios::badbit | std::ios::failbit);
        summary << "held_frames\t" << held.size() << "\nclosed_windows\t" << closed_windows
                << "\ndispatcher_completed\t1\npositive_controls\t" << controls << '\n';
        summary.close();
        return controls ? 0 : 5;
    }
    catch (const winrt::hresult_error& error) { std::cout << "hresult=" << error.code().value << std::endl; return 3; }
    catch (const std::exception& error) { std::cout << "harness=" << error.what() << std::endl; return 3; }
}
