// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Unmodified U source in a separate process: its mapped-view lifetime is unsafe.
#include "Screencap/DesktopDupWindowScreencap.h"
#include "print-thread.h"
#include <filesystem>
#include <fstream>
#include <iostream>
#include <chrono>
#include <array>
#include <algorithm>
#include <climits>
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
bool known(const cv::Mat& image, bool inside = false)
{
    if (image.cols < 16 || image.rows < 16 || image.type() != CV_8UC3
        || (!inside && (image.cols != 96 || image.rows != 64))) { return false; }
    const auto marker=image.at<cv::Vec3b>(0,0);
    if (inside && marker!=cv::Vec3b(0x1a,0x2b,0x3c) && marker!=cv::Vec3b(0x4e,0x5f,0x60)) { return false; }
    for (int y=0; y<image.rows; ++y) for (int x=0; x<image.cols; ++x) {
        const cv::Vec3b expected = inside && x<16 && y<16 ? marker
            : x<image.cols/3 ? cv::Vec3b(0x1f,0x53,0xa7) : y<image.rows/2 ? cv::Vec3b(0xb9,0x4d,0x27) : cv::Vec3b(0x3b,0xe2,0x81);
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
    const bool fullscreen=mode==L"window-fullscreen";
    if ((mode != L"desktop" && mode != L"window" && mode != L"default-output" && !fullscreen) || !output.is_absolute() || !std::filesystem::is_directory(output)) { return 1; }
    try {
        Fixture fixture;
        const auto created = mfs_print_thread_fixture(96,64,0,0,0,0,MFS_RENDER_DISABLE_TRANSITIONS | MFS_RENDER_TOPMOST,&fixture.value);
        if (created != 0) { std::cout << "fixture_create=" << created << std::endl; return 2; }
        std::array<int64_t,31> fullscreen_setup {};
        if (fullscreen) {
            const std::array<int64_t,6> query {};
            if (mfs_print_thread_geometry(fixture.value.owner,query.data(),query.size(),fullscreen_setup.data(),fullscreen_setup.size())!=0) { return 2; }
            if (std::equal(fullscreen_setup.begin(),fullscreen_setup.begin()+4,fullscreen_setup.begin()+4)) {
                std::cout<<"MFS_MISSING_ENVIRONMENT:fullscreen_workarea"<<std::endl;return 77;
            }
            const std::array<int64_t,6> setup {1,WS_POPUP | WS_VISIBLE,fullscreen_setup[0],fullscreen_setup[1],
                fullscreen_setup[2]-fullscreen_setup[0],fullscreen_setup[3]-fullscreen_setup[1]};
            if (setup[4]<16 || setup[5]<16 || setup[4]>INT_MAX || setup[5]>INT_MAX
                || mfs_print_thread_geometry(fixture.value.owner,setup.data(),setup.size(),fullscreen_setup.data(),fullscreen_setup.size())!=0) { return 2; }
            if (fullscreen_setup[16]!=fullscreen_setup[0] || fullscreen_setup[17]!=fullscreen_setup[1]
                || fullscreen_setup[14]!=setup[4] || fullscreen_setup[15]!=setup[5]) { return 5; }
        }
        const auto updates = fullscreen ? mfs_desktop_fixture_inside_updates(fixture.value.owner) : mfs_desktop_fixture_updates(fixture.value.owner);
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
        if (mode==L"window" || fullscreen) { MaaNS::CtrlUnitNs::DesktopDupWindowScreencap source(target); observe(source); }
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
        if (mode==L"window" || fullscreen) { target_pixels=retained; }
        else {
            const int64_t x=static_cast<int64_t>(origin.x)-monitor.rcMonitor.left, y=static_cast<int64_t>(origin.y)-monitor.rcMonitor.top;
            if (x<0 || y<0 || x+96>retained.cols || y+64>retained.rows) { return 77; }
            target_pixels=retained(cv::Rect(static_cast<int>(x),static_cast<int>(y),96,64));
        }
        save(target_pixels,output/"target.bgr");
        const bool pixels=known(target_pixels,fullscreen);
        std::cout << "known_target_pixels=" << pixels << std::endl;
        uint64_t marker[3] {};
        if (mfs_desktop_fixture_update_info(fixture.value.owner,marker,3)!=0) { return 2; }
        std::cout << "marker " << marker[0] << ' ' << marker[1] << ' ' << marker[2] << std::endl;
        if (fullscreen) {
            std::array<int64_t,31> captured {};
            const std::array<int64_t,6> query {};
            if (mfs_print_thread_geometry(fixture.value.owner,query.data(),query.size(),captured.data(),captured.size())!=0
                || captured[20]!=fullscreen_setup[20] || marker[0]!=reinterpret_cast<uintptr_t>(target)
                || target_pixels.cols!=static_cast<int64_t>(monitor.rcMonitor.right)-monitor.rcMonitor.left
                || target_pixels.rows!=static_cast<int64_t>(monitor.rcMonitor.bottom)-monitor.rcMonitor.top) { return 5; }
            std::ofstream evidence(output/"fullscreen.tsv",std::ios::binary);
            evidence.exceptions(std::ios::failbit | std::ios::badbit);
            for (const auto& snapshot : {fullscreen_setup,captured}) {
                for (size_t i=0;i<snapshot.size();++i) { if(i) { evidence<<'\t'; } evidence<<snapshot[i]; }
                evidence<<'\n';
            }
            evidence<<marker[0]<<'\t'<<marker[1]<<'\t'<<marker[2]<<'\n';
        }
        const auto copy=target_pixels.clone(); // Harness retention oracle only.
        if (!fixture.close()) { return 3; }
        if (cv::norm(copy,target_pixels,cv::NORM_INF)!=0) { return 5; }
        if (fullscreen) { save(target_pixels,output/"held-after-close.bgr"); }
        // The count includes the initial draw, not just timer callbacks.
        // Valid source acquisitions need not wait for an additional callback:
        // initial/pointer/other-window updates can satisfy them sooner.
        const bool passed=pixels && marker[0]!=0 && marker[1]>=1 && marker[2]==0;
        if (fullscreen && passed) {
            std::cout<<"original DesktopDupWindow fullscreen cases=1 captures=1 known_target_with_marker=1 work_area_crop=0 geometry_adjustments=0 retained_after_close=1 passed"<<std::endl;
        }
        return passed ? 0 : 5;
    }
    catch (const std::exception& e) { std::cerr << "source_or_harness_exception=" << e.what() << '\n'; return 3; }
    catch (...) { return 3; }
}
