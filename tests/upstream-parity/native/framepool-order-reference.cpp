// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Separate source-only order/lifetime observation. No native body is rewritten.
#define wmain mfs_unused_framepool_context_entry
#include "framepool-reference.cpp"
#undef wmain
#include "Screencap/DesktopDupWindowScreencap.h"
#include <cstdint>
#include <memory>
#include <vector>

namespace {
using Wgc = MaaNS::CtrlUnitNs::FramePoolScreencap;
using Dup = MaaNS::CtrlUnitNs::DesktopDupWindowScreencap;
struct Shot {
    unsigned arm=0,slot=0;
    std::string phase;
    bool wgc_object_alive=false,engaged=false;
    int64_t begin=0,end=0;
    WindowState before{},after{};
    cv::Mat image;
};
int64_t counter() {
    LARGE_INTEGER value{};
    if(!QueryPerformanceCounter(&value)) {throw std::runtime_error("QPC failed");}
    return value.QuadPart;
}
bool scene(const cv::Mat& image) {
    if(image.cols!=96 || image.rows!=64 || image.type()!=CV_8UC3) {return false;}
    const std::array<cv::Vec3b,3> colours={cv::Vec3b(0x1f,0x53,0xa7),cv::Vec3b(0xb9,0x4d,0x27),cv::Vec3b(0x3b,0xe2,0x81)};
    for(int y=0;y<64;++y) {for(int x=0;x<96;++x) {
        if(image.at<cv::Vec3b>(y,x)!=colours[x<32?0:y<32?1:2]) {return false;}
    }}
    return true;
}
template<class Source> void take(std::vector<Shot>& shots,unsigned arm,const std::string& phase,
    Source& source,HWND hwnd,bool alive) {
    if(shots.size()>std::numeric_limits<unsigned>::max()) {throw std::runtime_error("shot index overflow");}
    Shot shot;shot.arm=arm;shot.slot=static_cast<unsigned>(shots.size());shot.phase=phase;shot.wgc_object_alive=alive;
    shot.before=window_state(hwnd);shot.begin=counter();
    auto image=source.screencap(); // Exactly one unchanged source operation.
    shot.end=counter();shot.after=window_state(hwnd);shot.engaged=image.has_value();
    // Explicit diagnostic CPU snapshot: original results may alias caches.
    // This extra copy is neither a product optimization nor a native lease.
    if(image && !image->empty()) {shot.image=image->clone();}
    shots.push_back(std::move(shot));
}
void export_shot(const Shot& shot,const std::filesystem::path& root) {
    const auto dir=root/("shot-"+std::to_string(shot.slot));std::filesystem::create_directory(dir);
    std::ofstream bytes(dir/"image.bgr",std::ios::binary),layout(dir/"layout.tsv");
    bytes.exceptions(std::ios::badbit|std::ios::failbit);layout.exceptions(std::ios::badbit|std::ios::failbit);
    size_t row=0;
    if(!shot.image.empty()) {
        if(shot.image.type()!=CV_8UC3 || shot.image.cols<=0 || shot.image.rows<=0) {throw std::runtime_error("unexpected source layout");}
        const auto width=static_cast<size_t>(shot.image.cols);
        if(width>std::numeric_limits<size_t>::max()/3) {throw std::runtime_error("row overflow");}
        row=width*3;
        if(row>static_cast<size_t>(std::numeric_limits<std::streamsize>::max())) {throw std::runtime_error("stream width overflow");}
        for(int y=0;y<shot.image.rows;++y) {bytes.write(reinterpret_cast<const char*>(shot.image.ptr(y)),static_cast<std::streamsize>(row));}
    }
    layout<<shot.engaged<<'\t'<<shot.image.cols<<'\t'<<shot.image.rows<<'\t'<<shot.image.type()<<'\t'<<row<<'\n';
    bytes.close();layout.close();
}
}

int wmain(int argc,wchar_t** argv) {
    if(argc!=2 || !std::filesystem::is_directory(argv[1])) {return 64;}
    const std::filesystem::path root(argv[1]);
    try {
        Initialization initialization;winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED));initialization.owned=true;
        Dispatcher dispatcher;const DispatcherQueueOptions options{sizeof(DispatcherQueueOptions),DQTYPE_THREAD_CURRENT,DQTAT_COM_NONE};
        winrt::check_hresult(CreateDispatcherQueueController(options,reinterpret_cast<PDISPATCHERQUEUECONTROLLER*>(winrt::put_abi(dispatcher.controller))));
        const auto dpi=GetThreadDpiAwarenessContext();
        LARGE_INTEGER frequency{};if(!QueryPerformanceFrequency(&frequency)) {throw std::runtime_error("QPC frequency failed");}
        std::vector<Shot> shots;shots.reserve(40);
        std::array<std::array<int64_t,2>,4> destruction{};
        std::array<std::array<uint64_t,5>,4> targets{};
        std::array<std::array<uint64_t,10>,4> drivers{};
        unsigned closed_windows=0;
        // Arms0/3: no WGC. Arm1: DD warmup/test then WGC. Arm2: WGC first.
        // Each uses a fresh owned window and fresh source objects. No sleep or
        // retry follows WGC destruction; subsequent DD calls are observations.
        for(unsigned arm=0;arm<4;++arm) {
            Fixture fixture;
            const auto code=mfs_print_thread_fixture(96,64,0,0,0,0,MFS_RENDER_DISABLE_TRANSITIONS|MFS_RENDER_TOPMOST,&fixture.value);
            if(code!=0) {throw std::runtime_error("owned window creation failed");}
            const auto hwnd=static_cast<HWND>(fixture.value.hwnd);
            targets[arm][0]=reinterpret_cast<uintptr_t>(hwnd);
            if(mfs_print_thread_info(fixture.value.owner,targets[arm].data()+1,4)!=0) {throw std::runtime_error("native fixture identity failed");}
            // Desktop Duplication only produces a next frame after a desktop
            // update. Drive the existing nonoverlapping marker from its UI
            // owner, without changing target pixels or retrying capture. It
            // starts before any source object and runs throughout this arm;
            // no preparation/wait is inserted after WGC object destruction.
            if(mfs_desktop_fixture_updates(fixture.value.owner)!=0) {throw std::runtime_error("desktop frame driver setup failed");}
            std::array<uint64_t,3> updates{};
            if(mfs_desktop_fixture_update_info(fixture.value.owner,updates.data(),updates.size())!=0 || !updates[0] || !updates[1] || updates[2]) {throw std::runtime_error("desktop frame driver initial state failed");}
            const auto marker=reinterpret_cast<HWND>(static_cast<uintptr_t>(updates[0]));
            DWORD marker_pid=0;const auto marker_thread=GetWindowThreadProcessId(marker,&marker_pid);
            RECT marker_rect{},target_rect{},overlap{};
            const auto marker_monitor=MonitorFromWindow(marker,MONITOR_DEFAULTTONEAREST);
            const auto target_monitor=MonitorFromWindow(hwnd,MONITOR_DEFAULTTONEAREST);
            if(!marker_thread || marker_thread!=targets[arm][1] || marker_pid!=targets[arm][4] || marker==hwnd ||
                !GetWindowRect(marker,&marker_rect) || !GetWindowRect(hwnd,&target_rect) ||
                !marker_monitor || marker_monitor!=target_monitor || IntersectRect(&overlap,&marker_rect,&target_rect)) {throw std::runtime_error("desktop frame driver owner/output/isolation failed");}
            auto& driver=drivers[arm];
            driver={arm,updates[0],marker_thread,marker_pid,updates[1],0,0,1,1,0};
            {
                Dup duplicate(hwnd);std::unique_ptr<Wgc> wgc;
                if(arm==2) {
                    wgc=std::make_unique<Wgc>(hwnd);
                    take(shots,arm,"wgc-warm",*wgc,hwnd,true);take(shots,arm,"wgc-test",*wgc,hwnd,true);
                }
                take(shots,arm,"dd-warm",duplicate,hwnd,bool(wgc));take(shots,arm,"dd-test",duplicate,hwnd,bool(wgc));
                if(arm==1) {
                    wgc=std::make_unique<Wgc>(hwnd);
                    take(shots,arm,"wgc-warm",*wgc,hwnd,true);take(shots,arm,"wgc-test",*wgc,hwnd,true);
                    take(shots,arm,"dd-with-wgc",duplicate,hwnd,true);
                }
                destruction[arm][0]=counter();wgc.reset();destruction[arm][1]=counter();
                for(unsigned call=0;call<3;++call) {take(shots,arm,"dd-post-object-"+std::to_string(call),duplicate,hwnd,false);}
            }
            if(mfs_desktop_fixture_update_info(fixture.value.owner,updates.data(),updates.size())!=0 || updates[0]!=driver[1] || updates[1]<=driver[4] || updates[2]) {throw std::runtime_error("desktop frame driver progress failed");}
            driver[5]=updates[1];driver[6]=updates[2];
            if(!fixture.close()) {return 3;}++closed_windows;
            if(IsWindow(marker)) {throw std::runtime_error("desktop frame driver did not close");}driver[9]=1;
        }
        const bool dispatcher_within_budget=dispatcher.close();
        const bool dpi_same=AreDpiAwarenessContextsEqual(dpi,GetThreadDpiAwarenessContext());
        // Export after all source/window/dispatcher owners end. Extra property
        // queries or file IO cannot delay the first post-destruction DD call.
        std::ofstream rows(root/"shots.tsv"),states(root/"states.tsv"),owners(root/"owners.tsv"),windows(root/"windows.tsv"),driver_rows(root/"frame-drivers.tsv");
        rows.exceptions(std::ios::badbit|std::ios::failbit);states.exceptions(std::ios::badbit|std::ios::failbit);owners.exceptions(std::ios::badbit|std::ios::failbit);windows.exceptions(std::ios::badbit|std::ios::failbit);driver_rows.exceptions(std::ios::badbit|std::ios::failbit);
        rows<<"slot\tarm\tphase\twgc_object_alive\tbegin_qpc\tend_qpc\tengaged\twidth\theight\tknown_scene\tfnv64\n";
        bool controls=dispatcher_within_budget && dpi_same;
        for(const auto& shot:shots) {
            export_shot(shot,root);const bool known=scene(shot.image);
            rows<<shot.slot<<'\t'<<shot.arm<<'\t'<<shot.phase<<'\t'<<shot.wgc_object_alive<<'\t'<<shot.begin<<'\t'<<shot.end<<'\t'<<shot.engaged<<'\t'<<shot.image.cols<<'\t'<<shot.image.rows<<'\t'<<known<<'\t'<<digest(shot.image)<<'\n';
            states<<shot.slot;for(auto v:shot.before) {states<<'\t'<<v;}for(auto v:shot.after) {states<<'\t'<<v;}states<<'\n';
            if(shot.arm==0 || shot.arm==3 || shot.phase.starts_with("wgc-")) {controls &= shot.engaged && known;}
        }
        for(unsigned arm=0;arm<4;++arm) {
            owners<<arm<<'\t'<<destruction[arm][0]<<'\t'<<destruction[arm][1]<<'\n';
            windows<<arm;for(auto value:targets[arm]) {windows<<'\t'<<value;}windows<<'\n';
            for(size_t i=0;i<drivers[arm].size();++i) {if(i) {driver_rows<<'\t';}driver_rows<<drivers[arm][i];}driver_rows<<'\n';
        }
        rows.close();states.close();owners.close();windows.close();driver_rows.close();
        std::ofstream summary(root/"summary.txt");summary.exceptions(std::ios::badbit|std::ios::failbit);
        summary<<"schema=2\nframe_driver=1\ncv_cn_shift="<<CV_CN_SHIFT<<"\ncv_8u="<<CV_8U<<"\ncv_8uc3="<<CV_8UC3<<"\nshots="<<shots.size()<<"\nclosed_windows="<<closed_windows<<"\ndispatcher_completed=1\ndispatcher_within_budget="<<dispatcher_within_budget<<"\ncaller_dpi_preserved="<<dpi_same<<"\npositive_controls="<<controls<<"\nqpc_frequency="<<frequency.QuadPart<<"\nsource_bodies_unchanged=1\nextra_snapshot_copy=1\npost_close_calls_are_retries=0\noriginal_failure_reclassified=0\nfull_P1_acceptance=0\n";summary.close();
        std::cout<<"source order diagnostic shots="<<shots.size()<<" windows_closed="<<closed_windows<<" positive_controls="<<controls<<" no_capture_retry=1 full_P1_acceptance=0 frame_driver=1 schema=2\n";
        return controls?0:5;
    }catch(const winrt::hresult_error& error) {std::cerr<<"source hresult="<<error.code().value<<'\n';return 3;}
    catch(const std::exception& error) {std::cerr<<"diagnostic failure="<<error.what()<<'\n';return 3;}
}
