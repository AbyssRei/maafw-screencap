// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Unmodified U source in a separate process: its mapped-view lifetime is unsafe.
#include "Screencap/DesktopDupWindowScreencap.h"
#include "print-thread.h"
#include <filesystem>
#include <fstream>
#include <iostream>
#include <chrono>
#include <stdexcept>
#include <opencv2/core.hpp>
#include <wrl/client.h>
namespace {
struct Fixture {
    MfsPrintFixture value {};
    bool failed = false;
    bool close() {
        if (!value.owner) { return true; }
        if (failed) { return false; }
        failed = mfs_print_thread_close(&value.owner) != 0;
        return !failed;
    }
    ~Fixture() { if (value.owner && !close()) { std::terminate(); } }
};
void save(const cv::Mat& image, const std::filesystem::path& path)
{
    if (image.empty() || image.type() != CV_8UC3 || image.cols <= 0 || image.rows <= 0) { throw std::runtime_error("source layout"); }
    std::ofstream file(path,std::ios::binary);
    file.exceptions(std::ios::failbit | std::ios::badbit);
    for (int y=0; y<image.rows; ++y) { file.write(reinterpret_cast<const char*>(image.ptr(y)), static_cast<std::streamsize>(image.cols)*3); }
}
bool known(const cv::Mat& image)
{
    if (image.cols != 96 || image.rows != 64 || image.type() != CV_8UC3) { return false; }
    for (int y=0; y<64; ++y) for (int x=0; x<96; ++x) {
        const cv::Vec3b expected = x<32 ? cv::Vec3b(0x1f,0x53,0xa7) : y<32 ? cv::Vec3b(0xb9,0x4d,0x27) : cv::Vec3b(0x3b,0xe2,0x81);
        if (image.at<cv::Vec3b>(y,x) != expected) { return false; }
    }
    return true;
}
}
int wmain(int argc, wchar_t** argv)
{
    if (argc != 3) { return 1; }
    const std::wstring_view mode(argv[1]);
    const std::filesystem::path output(argv[2]);
    if ((mode != L"desktop" && mode != L"window" && mode != L"default-output") || !output.is_absolute() || !std::filesystem::is_directory(output)) { return 1; }
    try {
        Fixture fixture;
        const auto created = mfs_print_thread_fixture(96,64,0,0,0,0,MFS_RENDER_DISABLE_TRANSITIONS | MFS_RENDER_TOPMOST,&fixture.value);
        if (created != 0) { std::cout << "fixture_create=" << created << std::endl; return 2; }
        const auto updates = mfs_desktop_fixture_updates(fixture.value.owner);
        if (updates != 0) { std::cout << "fixture_updates=" << updates << std::endl; return updates==5 ? 77 : 2; }
        HWND target = static_cast<HWND>(fixture.value.hwnd);
        MONITORINFO monitor {sizeof(MONITORINFO)};
        POINT origin {};
        RECT client {};
        if (!GetMonitorInfoW(MonitorFromWindow(target,MONITOR_DEFAULTTONEAREST),&monitor) || !GetClientRect(target,&client) || !ClientToScreen(target,&origin)) { return 2; }
        if (mode==L"default-output") {
            // Independent preflight: upstream's "primary" means adapter 0 /
            // output 0, which is not assumed to be the OS primary monitor.
            Microsoft::WRL::ComPtr<IDXGIFactory> factory;
            Microsoft::WRL::ComPtr<IDXGIAdapter> adapter;
            Microsoft::WRL::ComPtr<IDXGIOutput> selected;
            DXGI_OUTPUT_DESC description {};
            if (FAILED(CreateDXGIFactory(IID_PPV_ARGS(factory.GetAddressOf()))) ||
                FAILED(factory->EnumAdapters(0,adapter.GetAddressOf())) ||
                FAILED(adapter->EnumOutputs(0,selected.GetAddressOf())) ||
                FAILED(selected->GetDesc(&description))) { return 2; }
            monitor.rcMonitor=description.DesktopCoordinates;
            const auto& r=monitor.rcMonitor;
            std::cout << "default_output " << r.left << ' ' << r.top << ' ' << r.right << ' ' << r.bottom
                << " rotation=" << description.Rotation << " attached=" << description.AttachedToDesktop << std::endl;
            if (!description.AttachedToDesktop || origin.x<r.left || origin.y<r.top ||
                static_cast<int64_t>(origin.x)+96>r.right || static_cast<int64_t>(origin.y)+64>r.bottom) {
                std::cout << "MFS_MISSING_ENVIRONMENT:default_output_fixture" << std::endl;
                return 77;
            }
        }
        cv::Mat retained;
        const auto observe = [&](auto& source) {
            const auto start = std::chrono::steady_clock::now();
            auto image = source.screencap(); // One call, including original warmups.
            std::cout << "source_result=" << (image ? "image" : "empty") << " elapsed_ms=" << std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now()-start).count() << std::endl;
            if (image) { retained = std::move(*image); }
        };
        if (mode==L"window") { MaaNS::CtrlUnitNs::DesktopDupWindowScreencap source(target); observe(source); }
        else { MaaNS::CtrlUnitNs::DesktopDupScreencap source(mode==L"default-output" ? nullptr : target); observe(source); }
        if (retained.empty()) { return 4; }
        save(retained,output/"image.bgr");
        std::cout << "source_image " << retained.cols << ' ' << retained.rows << ' ' << retained.step[0] << std::endl;
        std::ofstream geometry(output/"geometry.txt");
        geometry.exceptions(std::ios::failbit | std::ios::badbit);
        geometry << retained.cols << ' ' << retained.rows << ' ' << retained.step[0] << '\n'
            << monitor.rcMonitor.left << ' ' << monitor.rcMonitor.top << ' ' << monitor.rcMonitor.right << ' ' << monitor.rcMonitor.bottom << '\n'
            << origin.x << ' ' << origin.y << ' ' << client.right-client.left << ' ' << client.bottom-client.top << '\n';
        cv::Mat target_pixels;
        if (mode==L"window") { target_pixels=retained; }
        else {
            const int64_t x=static_cast<int64_t>(origin.x)-monitor.rcMonitor.left, y=static_cast<int64_t>(origin.y)-monitor.rcMonitor.top;
            if (x<0 || y<0 || x+96>retained.cols || y+64>retained.rows) { return 77; }
            target_pixels=retained(cv::Rect(static_cast<int>(x),static_cast<int>(y),96,64));
        }
        save(target_pixels,output/"target.bgr");
        const bool pixels=known(target_pixels);
        std::cout << "known_target_pixels=" << pixels << std::endl;
        uint64_t marker[3] {};
        if (mfs_desktop_fixture_update_info(fixture.value.owner,marker,3)!=0) { return 2; }
        std::cout << "marker " << marker[0] << ' ' << marker[1] << ' ' << marker[2] << std::endl;
        const auto copy=target_pixels.clone(); // Harness retention oracle only.
        if (!fixture.close()) { return 3; }
        if (cv::norm(copy,target_pixels,cv::NORM_INF)!=0) { return 5; }
        return pixels && marker[1]>1 && marker[2]==0 ? 0 : 5;
    }
    catch (const std::exception& e) { std::cerr << "source_or_harness_exception=" << e.what() << '\n'; return 3; }
    catch (...) { return 3; }
}
