// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Repeated target destruction through the unchanged fixed FramePool class.
#include "Screencap/FramePoolScreencap.h"
#include "print-thread.h"
#include <roapi.h>
#include <array>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <vector>

namespace {
struct Initialization {
    bool owned = false;
    ~Initialization() { if (owned) { RoUninitialize(); } }
};
struct Fixture {
    MfsPrintFixture value {};
    bool failed = false;
    bool close()
    {
        if (!value.owner) { return true; }
        if (failed) { return false; }
        failed = mfs_print_thread_close(&value.owner) != 0;
        return !failed;
    }
    ~Fixture() { if (value.owner && !close()) { std::terminate(); } }
};
void known(const cv::Mat& image)
{
    if (image.type() != CV_8UC3 || image.cols != 96 || image.rows != 64) {
        throw std::runtime_error("unexpected source target-loss image layout");
    }
    for (int y = 8; y < 56; ++y) {
        for (int x = 8; x < 88; ++x) {
            const auto expected = x < 32 ? cv::Vec3b(0x1f, 0x53, 0xa7)
                : y < 32 ? cv::Vec3b(0xb9, 0x4d, 0x27) : cv::Vec3b(0x3b, 0xe2, 0x81);
            if (image.at<cv::Vec3b>(y, x) != expected) { throw std::runtime_error("source target-loss known interior differs"); }
        }
    }
}
void save(const cv::Mat& image, const std::filesystem::path& root, const std::string& name)
{
    known(image);
    std::ofstream layout(root / (name + ".txt"), std::ios::binary);
    layout.exceptions(std::ios::badbit | std::ios::failbit);
    layout << image.cols << '\t' << image.rows << '\t' << image.step[0] << '\t' << 96 * 64 * 3 << '\n';
    std::ofstream pixels(root / (name + ".bgr"), std::ios::binary);
    pixels.exceptions(std::ios::badbit | std::ios::failbit);
    for (int y = 0; y < image.rows; ++y) {
        pixels.write(reinterpret_cast<const char*>(image.ptr(y)), 96 * 3);
    }
}
}

int wmain(int argc, wchar_t** argv)
{
    if (argc != 2) { return 1; }
    const std::filesystem::path root(argv[1]);
    if (!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root)) { return 1; }
    try {
        APTTYPE apartment {};
        APTTYPEQUALIFIER qualifier {};
        if (CoGetApartmentType(&apartment, &qualifier) != CO_E_NOTINITIALIZED) {
            throw std::runtime_error("target-loss probe requires a fresh caller process");
        }
        Initialization initialization;
        winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED));
        initialization.owned = true;
        std::ofstream rows(root / "target-loss.tsv", std::ios::binary);
        rows.exceptions(std::ios::badbit | std::ios::failbit);
        rows << "cycle\tinvalid_window\trejected_calls\twidth\theight\tfixture_closed\tsource_destroyed\tcaller_thread\tfixture_thread\telapsed_ms\n";
        std::vector<cv::Mat> held;
        held.reserve(20);
        for (int cycle = 0; cycle < 20; ++cycle) {
            Fixture fixture;
            if (mfs_print_thread_fixture(96, 64, 0, 0, 0, 0,
                    MFS_RENDER_DISABLE_TRANSITIONS | MFS_RENDER_TOPMOST, &fixture.value) != 0) {
                throw std::runtime_error("target-loss fixture creation failed");
            }
            std::array<uint64_t, 4> owner {};
            if (mfs_print_thread_info(fixture.value.owner, owner.data(), owner.size()) != 0
                || !owner[0] || owner[0] != owner[2] || owner[0] == owner[1]) {
                throw std::runtime_error("target-loss fixture owner differs");
            }
            const HWND target = static_cast<HWND>(fixture.value.hwnd);
            long long elapsed = 0;
            {
                MaaNS::CtrlUnitNs::FramePoolScreencap source(target);
                auto first = source.screencap(); // One initial capture; no retries.
                if (!first) { throw std::runtime_error("source initial target-loss capture failed"); }
                save(*first, root, std::to_string(cycle));
                held.push_back(std::move(*first));
                if (!fixture.close() || IsWindow(target)) {
                    throw std::runtime_error("owned target destruction/join failed");
                }
                const auto begin = std::chrono::steady_clock::now();
                // Three intentional calls on the same destroyed target. None
                // may return the cached image; errors are never retried away.
                for (int call = 0; call < 3; ++call) {
                    if (source.screencap()) { throw std::runtime_error("source returned an image after target destruction"); }
                }
                elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - begin).count();
            } // Actual original destructor/uninit before recording completion.
            rows << cycle << "\t1\t3\t96\t64\t1\t1\t" << owner[1] << '\t' << owner[0] << '\t' << elapsed << '\n';
            rows.flush();
        }
        RoUninitialize();
        initialization.owned = false;
        for (size_t cycle = 0; cycle < held.size(); ++cycle) {
            save(held[cycle], root, std::to_string(cycle) + "-held");
        }
        std::cout << "original FramePool target_loss cycles=20 captures=20 rejected_calls=60 retained=20 fixture_joins=20 source_destructors=20 passed\n";
        return 0;
    }
    catch (const winrt::hresult_error& error) {
        std::cerr << "target-loss HRESULT=" << error.code().value << '\n';
    }
    catch (const std::exception& error) { std::cerr << "target-loss failure=" << error.what() << '\n'; }
    return 1;
}
