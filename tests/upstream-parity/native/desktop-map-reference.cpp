// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Separate process: unchanged F retains its original expired mapped Mat view.
#include "Base/UnitBase.h"
#include "Common/Conf.h"
#include "Screencap/SafeDXGI.hpp"
#include "Screencap/HwndUtils.hpp"
#include "Screencap/DesktopDupScreencap.h"
#include "MaaUtils/ImageIo.h"
#include "MaaUtils/Logger.h"
#include "print-thread.h"
#include <array>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>
#include <dwmapi.h>
#include "desktop-map-probe.hpp"
// Logging also sees expression views; print their unchanged native addresses.
#pragma push_macro("VAR_VOIDP")
#undef VAR_VOIDP
#define VAR_VOIDP(value) #value << "=" << reinterpret_cast<void*>(::mfs_dup_map::pointer(value))
#define d3d_context_ ::mfs_dup_map::context(d3d_context_)
#define dxgi_dup_ ::mfs_dup_map::duplication(dxgi_dup_)
#define bgra_to_bgr ::mfs_dup_map::convert
#include MFS_UPSTREAM_DESKTOP_DUP_CPP
#undef bgra_to_bgr
#undef dxgi_dup_
#undef d3d_context_
#pragma pop_macro("VAR_VOIDP")

namespace {
struct Source : MAA_CTRL_UNIT_NS::DesktopDupScreencap {
    using DesktopDupScreencap::DesktopDupScreencap;
    bool warm()const{return !output_just_initialized_ && dxgi_dup_ && readable_texture_;}
};
struct Fixture {
    MfsPrintFixture value{};
    void close(){if(value.owner && mfs_print_thread_close(&value.owner))throw std::runtime_error("desktop fixture close failed");}
    ~Fixture(){if(value.owner && mfs_print_thread_close(&value.owner))std::terminate();}
};
int known(const cv::Mat& image) {
    if(image.type()!=CV_8UC3 || image.cols!=96 || image.rows!=64)throw std::runtime_error("desktop target layout differs");
    const auto first=image.at<cv::Vec3b>(0,0);
    const int phase=first==cv::Vec3b(78,95,96)?0:first==cv::Vec3b(26,43,60)?1:-1;
    if(phase<0)throw std::runtime_error("desktop target marker differs");
    for(int y=0;y<64;++y)for(int x=0;x<96;++x){
        const auto color=x<16 && y<16 ? (phase?cv::Vec3b(26,43,60):cv::Vec3b(78,95,96)) : x<32?cv::Vec3b(31,83,167):y<32?cv::Vec3b(185,77,39):cv::Vec3b(59,226,129);
        if(image.at<cv::Vec3b>(y,x)!=color)throw std::runtime_error("desktop target pixels differ");
    }
    return phase;
}
void save(const std::filesystem::path& root,const std::string& name,const cv::Mat& image) {
    std::ofstream layout(root/(name+".txt"),std::ios::binary),bytes(root/(name+".bgr"),std::ios::binary);
    layout.exceptions(std::ios::badbit|std::ios::failbit);bytes.exceptions(std::ios::badbit|std::ios::failbit);
    layout<<image.cols<<'\t'<<image.rows<<'\t'<<image.step[0]<<"\t18432\n";
    for(int y=0;y<image.rows;++y)bytes.write(reinterpret_cast<const char*>(image.ptr(y)),288);
    bytes.flush();layout.flush();known(image);
}
cv::Mat target(const cv::Mat& desktop,HWND window) {
    RECT client{};POINT origin{};MONITORINFO monitor{sizeof(MONITORINFO)};
    if(!GetClientRect(window,&client) || client.right!=96 || client.bottom!=64 || !ClientToScreen(window,&origin) || !GetMonitorInfoW(MonitorFromWindow(window,MONITOR_DEFAULTTONEAREST),&monitor))throw std::runtime_error("desktop target geometry failed");
    const int64_t x=static_cast<int64_t>(origin.x)-monitor.rcMonitor.left,y=static_cast<int64_t>(origin.y)-monitor.rcMonitor.top;
    if(x<0 || y<0 || x+96>desktop.cols || y+64>desktop.rows)throw std::runtime_error("desktop target outside output");
    return desktop(cv::Rect(static_cast<int>(x),static_cast<int>(y),96,64));
}
}
int wmain(int argc,wchar_t** argv) {
    try {
        if(argc!=2)throw std::runtime_error("usage: desktop-map EMPTY_ABSOLUTE_ROOT");
        const std::filesystem::path root(argv[1]);
        if(!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root))throw std::runtime_error("desktop-map root must be empty");
        std::ofstream rows(root/"map.tsv",std::ios::binary);
        rows.exceptions(std::ios::badbit|std::ios::failbit);
        rows<<"cycle\tfault\tphase\toutcome\torder\tacquires\tcopies\tmaps\tunmaps\treleases\tconversions\tacquired\tmapped_result\treleased\ttimeout\tsubresource\tpitch\tthread\tcontext\tresource\tduplication\tmarker\n";
        struct Held{std::string name;cv::Mat image;};std::vector<Held> held;held.reserve(60);
        unsigned native_failures=0,conversion_failures=0;
        for(uint32_t cycle=0;cycle<20;++cycle){
            Fixture fixture;
            if(mfs_print_thread_fixture(96,64,0,0,0,0,MFS_RENDER_DISABLE_TRANSITIONS|MFS_RENDER_TOPMOST,&fixture.value) || mfs_desktop_fixture_inside_updates(fixture.value.owner))throw std::runtime_error("desktop-map fixture setup failed");
            {
                Source source(static_cast<HWND>(fixture.value.hwnd));
                auto warm=source.screencap();
                if(!warm || !source.warm())throw std::runtime_error("desktop-map initial capture did not establish warm state");
                auto warm_target=target(*warm,static_cast<HWND>(fixture.value.hwnd));
                save(root,std::to_string(cycle)+"-warm",warm_target);held.push_back({std::to_string(cycle)+"-warm-held",warm_target});
                for(const auto phase:{"probe","recovery"}){
                    const uint32_t fault=std::string_view(phase)=="probe"?cycle%3:0;
                    mfs_dup_map::Trace trace;trace.fault=fault;
                    std::optional<cv::Mat> image;bool conversion_error=false;
                    {mfs_dup_map::Scope scope(trace);try{image=source.screencap();}catch(const mfs_dup_map::ConversionFailure&){conversion_error=true;}}
                    if(trace.violation || trace.mapped || trace.held || trace.order!=(fault==1?1235u:123456u)
                        || trace.acquires!=1 || trace.copies!=1 || trace.maps!=1 || trace.unmaps!=(fault==1?0u:1u) || trace.releases!=1 || trace.conversions!=(fault==1?0u:1u)
                        || trace.acquired<0 || trace.released<0 || trace.timeout!=2000 || trace.subresource!=(fault==1?UINT32_MAX:0) || !trace.context || !trace.resource || !trace.duplication || trace.thread!=GetCurrentThreadId()
                        || (fault==1?(trace.mapped_result>=0 || trace.pitch!=0):(trace.mapped_result<0 || trace.pitch<384))
                        || bool(image)!=(fault==0) || conversion_error!=(fault==2) || !source.warm())throw std::runtime_error("desktop-map native result/order/cleanup differs");
                    int marker=-1;
                    if(image){auto roi=target(*image,static_cast<HWND>(fixture.value.hwnd));const auto name=std::to_string(cycle)+"-"+phase;save(root,name,roi);marker=known(roi);held.push_back({name+"-held",roi});}
                    if(fault==1)++native_failures;if(fault==2)++conversion_failures;
                    rows<<cycle<<'\t'<<fault<<'\t'<<phase<<'\t'<<(fault==0?1:fault==1?0:2)<<'\t'<<trace.order<<'\t'<<trace.acquires<<'\t'<<trace.copies<<'\t'<<trace.maps<<'\t'<<trace.unmaps<<'\t'<<trace.releases<<'\t'<<trace.conversions<<'\t'<<trace.acquired<<'\t'<<trace.mapped_result<<'\t'<<trace.released<<'\t'<<trace.timeout<<'\t'<<trace.subresource<<'\t'<<trace.pitch<<'\t'<<trace.thread<<'\t'<<trace.context<<'\t'<<trace.resource<<'\t'<<trace.duplication<<'\t'<<marker<<'\n';rows.flush();
                }
            }
            fixture.close();
        }
        for(const auto& entry:held)save(root,entry.name,entry.image);
        if(native_failures!=7 || conversion_failures!=6 || held.size()!=47)throw std::runtime_error("desktop-map matrix incomplete");
        std::cout<<"original DesktopDup map_boundary cycles=20 warm=20 observed_calls=40 map_failures=7 conversion_failures=6 recovery=20 retained=47 native_forwarded=1 full_P1_acceptance=0 passed\n";
        return 0;
    }catch(const std::exception& error){std::cerr<<"desktop-map failure="<<error.what()<<'\n';return 1;}
}
