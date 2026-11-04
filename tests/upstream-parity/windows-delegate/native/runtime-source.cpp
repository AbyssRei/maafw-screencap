// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Explicit native acceptance process; capture remains synchronous and unchanged.
#include "bridge.h"
#include "Manager/GamepadControlUnitMgr.h"
#include "MaaControlUnit/GamepadControlUnitAPI.h"
#include "MaaUtils/Runtime.h"
#include <dispatcherqueue.h>
#include <roapi.h>
#include <winrt/Windows.System.h>
#include <winrt/Windows.Foundation.h>
#include <chrono>
#include <fstream>
#include <iostream>
#include <thread>
#include <vector>
#include <array>
#include <algorithm>
#include <limits>
#include "gamepad-runtime.hpp"
#include "whole-gamepad-runtime.hpp"
namespace {
struct Host {
    bool initialized=false;
    bool attempted=false;
    winrt::Windows::System::DispatcherQueueController controller{nullptr};
    Host() {
        winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED));initialized=true;
        try {
            const DispatcherQueueOptions options{sizeof(DispatcherQueueOptions),DQTYPE_THREAD_CURRENT,DQTAT_COM_NONE};
            winrt::check_hresult(CreateDispatcherQueueController(options,reinterpret_cast<PDISPATCHERQUEUECONTROLLER*>(winrt::put_abi(controller))));
        }catch(...) {RoUninitialize();initialized=false;throw;}
    }
    void close() {
        if(!controller) {return;}
        if(attempted) {std::abort();}attempted=true;
        auto operation=controller.ShutdownQueueAsync();const auto began=std::chrono::steady_clock::now();bool late=false;
        while(operation.Status()==winrt::Windows::Foundation::AsyncStatus::Started) {
            MSG message{};
            while(PeekMessageW(&message,nullptr,0,0,PM_REMOVE)) {
                if(message.message==WM_QUIT) {std::abort();}
                TranslateMessage(&message);DispatchMessageW(&message);
            }
            late|=std::chrono::steady_clock::now()-began>std::chrono::seconds(5);
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
        operation.GetResults();controller=nullptr;
        if(late) {throw std::runtime_error("dispatcher completion exceeded observation budget");}
    }
    ~Host() {
        try {close();}catch(...) {std::cerr<<"unproved dispatcher shutdown"<<std::endl;std::abort();}
        if(initialized) {RoUninitialize();}
    }
};
// Separate actual UI thread/WindowProc from the independently prepared common
// rendering adapter. This load is acceptance infrastructure, not the original
// unit loader; its search flags never alter process-wide search settings.
struct Fixture {
    struct Span {void* owner;void* hwnd;void* popup;int32_t width,height,popup_width,popup_height;} span{};
    HMODULE module=nullptr;
    using Close=int32_t(*)(void**);
    Close release=nullptr;bool attempted=false;
    template<class T> T symbol(const char* name) {
        auto pointer=GetProcAddress(module,name);if(!pointer) {throw std::runtime_error("render adapter export missing");}return reinterpret_cast<T>(pointer);
    }
    void open(const std::filesystem::path& path,uint64_t method) {
        if(!path.is_absolute()) {throw std::logic_error("absolute renderer path required");}
        module=LoadLibraryExW(path.c_str(),nullptr,LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_SYSTEM32);
        if(!module) {throw std::runtime_error("render adapter load failed");}
        release=symbol<Close>("mfs_print_thread_close");
        using Layout=int32_t(*)(uint64_t*,size_t);
        uint64_t layout[11]{};
        if(symbol<Layout>("mfs_print_parity_layout")(layout,11)) {throw std::runtime_error("renderer layout query");}
        const uint64_t expected[]={sizeof(Span),alignof(Span),offsetof(Span,owner),offsetof(Span,hwnd),offsetof(Span,popup),offsetof(Span,width),offsetof(Span,height),offsetof(Span,popup_width),offsetof(Span,popup_height),0x100,0x200};
        if(!std::equal(std::begin(expected),std::end(expected),std::begin(layout))) {throw std::runtime_error("renderer layout mismatch");}
        using Create=int32_t(*)(int32_t,int32_t,int32_t,int32_t,int32_t,int32_t,uint32_t,Span*);
        const auto created=symbol<Create>("mfs_print_thread_fixture")(96,64,0,0,0,0,0x300,&span);
        if(created) {std::cerr<<"renderer create status="<<created<<std::endl;throw std::runtime_error("renderer create failed");}
        if(!span.owner || !span.hwnd || span.popup || span.width!=96 || span.height!=64) {throw std::runtime_error("renderer target mismatch");}
        if(method&12) {
            using Start=int32_t(*)(void*);
            if(symbol<Start>("mfs_desktop_fixture_updates")(span.owner)) {throw std::runtime_error("desktop update fixture failed");}
        }
    }
    void close() {
        if(!span.owner) {return;}if(attempted) {std::abort();}attempted=true;
        if(!release || release(&span.owner) || span.owner) {std::cerr<<"unproved fixture shutdown"<<std::endl;std::abort();}
    }
    ~Fixture() {close();if(module && !FreeLibrary(module)) {std::cerr<<"renderer module release failed"<<std::endl;std::abort();}}
};
struct Trace {
    std::filesystem::path output;
    ~Trace() {
        try {
            MfsDnEvent events[4096]{};size_t count=0;uint64_t owners=0;
            if(mfs_dn_events(events,std::size(events),&count,&owners)) {return;}
            std::ofstream stream(output/L"source-events.tsv");
            for(size_t index=0;index<count;++index) {const auto& e=events[index];stream<<e.kind<<'\t'<<e.thread<<'\t'<<e.a<<'\t'<<e.b<<'\t'<<e.c<<'\t'<<e.d<<'\n';}
        }catch(...) {std::cerr<<"source trace export failed"<<std::endl;}
    }
};
// This opt-in scene adds native nonclient space, not a target-pixel inset.
// Read caller-side Win32 geometry around public operations without pumping or
// synchronizing the UI queue; these are boundary observations, not atomic ones.
struct ClientBand {
    std::ofstream rows;
    std::filesystem::path root;
    std::array<int64_t,10> expected{};
    uint64_t count=0;
    bool valid=true;
    DPI_AWARENESS_CONTEXT previous=nullptr;
    static std::array<int64_t,10> snapshot(HWND window) {
        RECT outer{},client{};POINT origin{};
        if(!GetWindowRect(window,&outer) || !GetClientRect(window,&client) || !ClientToScreen(window,&origin)) {throw std::runtime_error("band native geometry query");}
        return {outer.left,outer.top,outer.right,outer.bottom,client.left,client.top,client.right,client.bottom,origin.x,origin.y};
    }
    ClientBand(Fixture& fixture,const std::filesystem::path& output):root(output) {
        previous=SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if(!previous) {throw std::runtime_error("band caller DPI setup");}
        try {
            using Query=int32_t(*)(void*,const int64_t*,size_t,int64_t*,size_t);
            const auto query=fixture.symbol<Query>("mfs_print_thread_geometry");
            std::array<int64_t,31> before{},after{};const int64_t inspect[6]{};
            if(query(fixture.span.owner,inspect,6,before.data(),before.size())) {throw std::runtime_error("band initial geometry");}
            const int64_t x=before[4]+64,y=before[5]+64;
            if(x<INT32_MIN+6LL || y<INT32_MIN+6LL || x>INT32_MAX-102LL || y>INT32_MAX-70LL) {throw std::runtime_error("band coordinate range");}
            const int64_t prepare[6]{4,0x94000000,x,y,96,64},paint[6]{2,0,0,0,0,0};
            if(query(fixture.span.owner,prepare,6,after.data(),after.size()) || query(fixture.span.owner,paint,6,after.data(),after.size())) {throw std::runtime_error("band native setup");}
            expected={x-6,y-6,x+102,y+70,0,0,96,64,x,y};
            if(!std::equal(before.begin(),before.begin()+8,after.begin()) || !std::equal(expected.begin(),expected.end(),after.begin()+8)
                || after[18]!=0x94000000 || !(after[19]&8) || after[27]!=1 || after[28]!=0) {throw std::runtime_error("band complete-client premise");}
            std::ofstream setup(root/L"client-band-setup.tsv");setup.exceptions(std::ios::badbit|std::ios::failbit);
            for(const auto& row:{before,after}) {for(size_t i=0;i<row.size();++i) {if(i) {setup<<'\t';}setup<<row[i];}setup<<'\n';}setup.close();
            rows.open(root/L"client-band-boundaries.tsv");rows.exceptions(std::ios::badbit|std::ios::failbit);
        }catch(...) {if(!SetThreadDpiAwarenessContext(previous)) {std::abort();}previous=nullptr;throw;}
    }
    void record(HWND window,uint64_t cycle,uint64_t operation,const std::array<int64_t,10>& before) {
        const auto after=snapshot(window);valid&=before==expected && after==expected;
        rows<<cycle<<'\t'<<operation;for(auto value:before) {rows<<'\t'<<value;}for(auto value:after) {rows<<'\t'<<value;}rows<<'\n';rows.flush();
        if(count==UINT64_MAX) {throw std::runtime_error("band record count overflow");}++count;
    }
    bool finish() {
        const bool complete=valid && count==120;rows.close();
        std::ofstream summary(root/L"client-band-summary.txt");summary.exceptions(std::ios::badbit|std::ios::failbit);
        summary<<"schema=1\nrows="<<count<<"\ngeometry_valid="<<complete<<"\nnonclient_band=6\ntarget_inset=0\n";summary.close();return complete;
    }
    ~ClientBand() {if(previous && !SetThreadDpiAwarenessContext(previous)) {std::abort();}}
};
struct Geometry {int32_t width=96,height=64,x=0,y=0;};
Geometry geometry(HWND window,uint64_t selected) {
    Geometry output;
    if(selected==4) {
        MONITORINFO info{};info.cbSize=sizeof(info);POINT origin{};
        if(!GetMonitorInfoW(MonitorFromWindow(window,MONITOR_DEFAULTTONEAREST),&info) || !ClientToScreen(window,&origin)) {throw std::runtime_error("runtime monitor geometry query");}
        const int64_t values[]={static_cast<int64_t>(info.rcMonitor.right)-info.rcMonitor.left,static_cast<int64_t>(info.rcMonitor.bottom)-info.rcMonitor.top,static_cast<int64_t>(origin.x)-info.rcMonitor.left,static_cast<int64_t>(origin.y)-info.rcMonitor.top};
        for(auto value:values) {if(value<0 || value>INT32_MAX) {throw std::runtime_error("runtime monitor geometry range");}}
        output={static_cast<int32_t>(values[0]),static_cast<int32_t>(values[1]),static_cast<int32_t>(values[2]),static_cast<int32_t>(values[3])};
    }
    if(output.width<=0 || output.height<=0 || static_cast<int64_t>(output.x)+96>output.width || static_cast<int64_t>(output.y)+64>output.height) {throw std::runtime_error("runtime target not contained in unrotated output");}return output;
}
void save(const cv::Mat& image,const std::filesystem::path& path,const Geometry& scope) {
    if(image.empty() || image.type()!=CV_8UC3 || image.cols!=scope.width || image.rows!=scope.height) {throw std::runtime_error("invalid actual runtime image layout");}
    const auto row=static_cast<uint64_t>(image.cols)*3;
    if(row>static_cast<uint64_t>(std::numeric_limits<std::streamsize>::max()) || image.step[0]<row) {throw std::runtime_error("runtime row span");}
    std::ofstream stream(path,std::ios::binary);
    for(int y=0;y<image.rows;++y) {stream.write(reinterpret_cast<const char*>(image.ptr(y)),static_cast<std::streamsize>(row));}
    stream.close();if(!stream) {throw std::runtime_error("runtime image write failed");}
}
bool pixels(const cv::Mat& image,const Geometry& scope) {
    for(int y=0;y<64;++y) {for(int x=0;x<96;++x) {
        const cv::Vec3b expected=x<32?cv::Vec3b(0x1f,0x53,0xa7):(y<32?cv::Vec3b(0xb9,0x4d,0x27):cv::Vec3b(0x3b,0xe2,0x81));
        if(image.at<cv::Vec3b>(scope.y+y,scope.x+x)!=expected) {return false;}
    }}return true;
}
}
int wmain(int argc,wchar_t** argv) {
    try {
        if(argc!=6 && argc!=7) {throw std::logic_error("expected directory renderer output mask cycles [client-band]");}
        const bool client_band=argc==7;
        const bool whole_chain=client_band && std::wstring_view(argv[6])==L"whole-client-band";
        if(client_band && !whole_chain && std::wstring_view(argv[6])!=L"client-band") {throw std::logic_error("unknown runtime scene");}
        MaaNS::directory=argv[1];const std::filesystem::path output=argv[3];
        const auto method=std::stoull(argv[4]);const auto cycles=std::stoul(argv[5]);
        if(!method || method>63 || cycles!=20 || !output.is_absolute() || !std::filesystem::is_directory(output)) {throw std::logic_error("runtime arguments");}
        if(std::string_view(MaaGamepadControlUnitGetVersion())!="mfs-p1-gamepad-native-runtime") {throw std::logic_error("original outer API version differs");}
        MaaGamepadControlUnitDestroy(nullptr);
        mfs_dn_set_mode(5);Trace trace{output};MfsGamepadNative::Trace outer_trace{output};Host host;Fixture fixture;fixture.open(argv[2],client_band?0:method);
        std::unique_ptr<ClientBand> band;
        if(client_band) {
            band=std::make_unique<ClientBand>(fixture,output);
            if(method&12) {using Start=int32_t(*)(void*);if(fixture.symbol<Start>("mfs_desktop_fixture_updates")(fixture.span.owner)) {throw std::runtime_error("band desktop update fixture failed");}}
        }
        const auto window=static_cast<HWND>(fixture.span.hwnd);
        std::unique_ptr<MfsWholeGamepad::Trace> chain;
        if(whole_chain) {chain=std::make_unique<MfsWholeGamepad::Trace>(MaaNS::directory,output);}
        std::vector<std::pair<cv::Mat,Geometry>> held;held.reserve(60);
        std::ofstream table(output/L"source-cycles.tsv");
        bool mismatch=false;
        for(unsigned cycle=0;cycle<cycles;++cycle) {
            uint64_t selected=0;Geometry scope;
            {
                MfsGamepadFixture::mode=0;
                std::unique_ptr<MAA_CTRL_UNIT_NS::GamepadControlUnitAPI,decltype(&MaaGamepadControlUnitDestroy)> manager(
                    MaaGamepadControlUnitCreate(fixture.span.hwnd,cycle%2,method),MaaGamepadControlUnitDestroy);
                if(!manager || manager->connected()) {throw std::runtime_error("source outer initial state");}
                if(chain) {chain->record(cycle,0,manager.get());}
                std::cout<<"source cycle="<<cycle<<" initialize"<<std::endl;
                auto before=band?ClientBand::snapshot(window):std::array<int64_t,10>{};
                const bool connected=manager->connect();
                if(band) {band->record(window,cycle,0,before);}
                if(!connected || !manager->connected()) {throw std::runtime_error("source runtime outer initialization false");}
                if(chain) {selected=chain->record(cycle,1,manager.get());}
                else {
                    MfsDnEvent events[4096]{};size_t count=0;uint64_t owners=0;
                    if(mfs_dn_events(events,std::size(events),&count,&owners)) {throw std::runtime_error("source runtime diagnostic overflow");}
                    for(size_t index=0;index<count;++index) {if(events[index].kind==7) {selected=events[index].a;}}
                }
                if(!selected || (selected&method)!=selected) {throw std::runtime_error("source runtime selected outside mask");}
                scope=geometry(static_cast<HWND>(fixture.span.hwnd),selected);
                for(unsigned shot=0;shot<3;++shot) {
                    if(shot==1) {
                        MfsGamepadFixture::mode=cycle%3+1;
                        before=band?ClientBand::snapshot(window):std::array<int64_t,10>{};
                        int status=2;try{status=manager->connect();}catch(const std::runtime_error&){}
                        if(band) {band->record(window,cycle,2,before);}
                        if(status!=(MfsGamepadFixture::mode==1?0:2) || manager->connected()) {throw std::runtime_error("source controlled outer failure differs");}
                        if(chain && chain->record(cycle,3,manager.get())!=selected) {throw std::runtime_error("actual inner selection changed after outer failure");}
                    }
                    cv::Mat image;
                    before=band?ClientBand::snapshot(window):std::array<int64_t,10>{};
                    if(!manager->screencap(image)) {throw std::runtime_error("source runtime capture false");}
                    if(chain && chain->record(cycle,shot==0?2:shot+3,manager.get())!=selected) {throw std::runtime_error("actual inner selection changed after capture");}
                    if(band) {band->record(window,cycle,shot==0?1:shot+2,before);}
                    // Extra executable-owned safety clone, before unit/module
                    // destruction. Native conversion remains at original point.
                    auto independent=image.clone();
                    save(independent,output/(L"source-"+std::to_wstring(cycle)+L"-"+std::to_wstring(shot)+L".bgr"),scope);
                    mismatch|=!pixels(independent,scope);held.emplace_back(std::move(independent),scope);
                }
                before=band?ClientBand::snapshot(window):std::array<int64_t,10>{};
                const bool inactive=manager->inactive();
                if(band) {band->record(window,cycle,5,before);}
                if(!inactive) {throw std::runtime_error("source runtime inactive false");}
                if(chain && chain->record(cycle,6,manager.get())!=selected) {throw std::runtime_error("actual inner selection changed after inactive");}
            }
            if(chain) {chain->record(cycle,7,nullptr);}
            const bool unloaded=GetModuleHandleW((MaaNS::directory/L"MaaWin32ControlUnit.dll").c_str())==nullptr;
            table<<cycle<<'\t'<<selected<<'\t'<<unloaded<<'\t'<<scope.width<<'\t'<<scope.height<<'\t'<<scope.x<<'\t'<<scope.y<<'\n';table.flush();
            if(!unloaded) {throw std::runtime_error("source runtime module retained");}
        }
        if(!MfsGamepadFixture::inputs.empty()) {throw std::runtime_error("source outer input owners survived");}
        const bool band_valid=!band || band->finish();
        if(chain) {chain->finish();}
        fixture.close();
        for(size_t index=0;index<held.size();++index) {
            mismatch|=!pixels(held[index].first,held[index].second);
            save(held[index].first,output/(L"source-held-"+std::to_wstring(index)+L".bgr"),held[index].second);
        }
        host.close();table.close();if(!table) {throw std::runtime_error("source runtime table write failed");}
        std::ofstream summary(output/L"source-summary.txt");summary<<"method="<<method<<"\ncycles=20\nheld=60\nwindow_closed=1\ndispatcher_closed=1\nknown_pixels="<<!mismatch<<'\n';summary.close();
        if(!summary || mismatch) {throw std::runtime_error("source runtime known pixels failed");}
        if(!band_valid) {throw std::runtime_error("source runtime band geometry failed");}
        return 0;
    }catch(const winrt::hresult_error& error) {std::cerr<<"source runtime HRESULT="<<error.code().value<<std::endl;return 2;}
    catch(const std::exception& error) {std::cerr<<error.what()<<std::endl;return 1;}
}
