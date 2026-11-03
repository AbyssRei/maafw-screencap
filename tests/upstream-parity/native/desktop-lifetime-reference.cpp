// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// The complete unedited F class; four real native lifetime observations.
#include "Screencap/DesktopDupScreencap.h"
#include "print-thread.h"
#include "d3d-lifetime.h"
#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>
namespace {
void require(bool value,const char* text){if(!value){throw std::runtime_error(text);}}
void status(int32_t value){require(value==0,"native lifetime observer failed");}
struct Source : MaaNS::CtrlUnitNs::DesktopDupScreencap {
    using DesktopDupScreencap::DesktopDupScreencap;
    std::array<void*,4> objects()const{return {d3d_device_,d3d_context_,dxgi_dup_,readable_texture_};}
    ID3D11Texture2D* texture()const{return readable_texture_;}
    ID3D11Device* device()const{return d3d_device_;}
    bool warm()const{return !output_just_initialized_ && readable_texture_ && dxgi_dup_;}
    unsigned slots()const{return unsigned(bool(d3d_device_))|(unsigned(bool(d3d_context_))<<1)|(unsigned(bool(dxgi_factory_))<<2)
        |(unsigned(bool(dxgi_adapter_))<<3)|(unsigned(bool(dxgi_output_))<<4)|(unsigned(bool(dxgi_dup_))<<5)|(unsigned(bool(readable_texture_))<<6);}
    void close(){uninit();}
};
struct Fixture {
    MfsPrintFixture value{};
    ~Fixture(){if(value.owner && mfs_print_thread_close(&value.owner)){std::terminate();}}
    void close(){if(value.owner){require(mfs_print_thread_close(&value.owner)==0,"window close failed");}}
};
std::array<uint64_t,12> read(uint32_t index){std::array<uint64_t,12> r{};status(mfs_d3d_lifetime_read(index,r.data(),r.size()));return r;}
void snapshot(std::ostream& rows,uint32_t cycle,uint32_t phase){
    for(uint32_t kind=0;kind<4;++kind){const auto r=read(cycle*4+kind);rows<<cycle<<'\t'<<phase<<'\t'<<kind;for(auto value:r){rows<<'\t'<<value;}rows<<'\n';}rows.flush();
}
void save(const std::filesystem::path& root,const std::string& name,const cv::Mat& image){
    require(image.type()==CV_8UC3 && image.rows==64 && image.cols==96,"target CPU layout differs");
    std::ofstream pixels(root/(name+".bgr"),std::ios::binary);pixels.exceptions(std::ios::badbit|std::ios::failbit);
    for(int y=0;y<64;++y){pixels.write(reinterpret_cast<const char*>(image.ptr(y)),288);}pixels.close();
    const auto first=image.at<cv::Vec3b>(0,0);
    const int phase=first==cv::Vec3b(78,95,96)?0:first==cv::Vec3b(26,43,60)?1:-1;
    require(phase>=0,"target marker differs");
    for(int y=0;y<64;++y){for(int x=0;x<96;++x){const auto expected=x<16 && y<16?(phase?cv::Vec3b(26,43,60):cv::Vec3b(78,95,96)):x<32?cv::Vec3b(31,83,167):y<32?cv::Vec3b(185,77,39):cv::Vec3b(59,226,129);require(image.at<cv::Vec3b>(y,x)==expected,"target pixels differ");}}
}
cv::Mat target(const cv::Mat& image,HWND window){
    RECT c{};POINT p{};MONITORINFO m{sizeof(MONITORINFO)};
    require(GetClientRect(window,&c) && c.right==96 && c.bottom==64 && ClientToScreen(window,&p)
        && GetMonitorInfoW(MonitorFromWindow(window,MONITOR_DEFAULTTONEAREST),&m),"target geometry failed");
    const int64_t x=static_cast<int64_t>(p.x)-m.rcMonitor.left,y=static_cast<int64_t>(p.y)-m.rcMonitor.top;
    require(x>=0 && y>=0 && x+96<=image.cols && y+64<=image.rows,"target outside captured monitor");
    return image(cv::Rect(static_cast<int>(x),static_cast<int>(y),96,64));
}
}
int wmain(int argc,wchar_t** argv){
    if(argc!=2){return 64;}const std::filesystem::path root(argv[1]);
    if(!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root)){return 64;}
    try {
        require(mfs_d3d_lifetime_version()==1,"observer version differs");
        std::array<uint32_t,6> widths{};status(mfs_d3d_lifetime_widths(widths.data(),widths.size()));require(widths==std::array<uint32_t,6>{8,8,8,4,4,4},"native ABI widths differ");
        std::ofstream abi(root/"abi.tsv",std::ios::binary);abi.exceptions(std::ios::badbit|std::ios::failbit);abi<<1;for(auto value:widths){abi<<'\t'<<value;}abi<<'\n';abi.close();
        std::ofstream rows(root/"lifetimes.tsv",std::ios::binary),calls(root/"calls.tsv",std::ios::binary);
        rows.exceptions(std::ios::badbit|std::ios::failbit);calls.exceptions(std::ios::badbit|std::ios::failbit);
        rows<<"cycle\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr\n";
        calls<<"cycle\theld\tbefore_slots\tafter_slots\tcaller\tbegin_close\tend_close\trelease_qpc\tfinal_qpc\n";
        std::vector<cv::Mat> held_images;held_images.reserve(20);
        for(uint32_t cycle=0;cycle<20;++cycle){
            Fixture fixture;require(!mfs_print_thread_fixture(96,64,0,0,0,0,MFS_RENDER_DISABLE_TRANSITIONS|MFS_RENDER_TOPMOST,&fixture.value)
                && !mfs_desktop_fixture_inside_updates(fixture.value.owner),"owned desktop scene failed");
            Source source(static_cast<HWND>(fixture.value.hwnd));auto frame=source.screencap();require(frame && source.warm(),"source did not establish warm capture");
            auto image=target(*frame,static_cast<HWND>(fixture.value.hwnd));save(root,std::to_string(cycle)+"-current",image);held_images.push_back(image);
            const auto objects=source.objects();for(uint32_t kind=0;kind<4;++kind){status(mfs_d3d_lifetime_attach(objects[kind],kind,4*cycle+kind));}
            snapshot(rows,cycle,0);for(uint32_t kind=0;kind<4;++kind){require(read(4*cycle+kind)[4]==0,"live native object notified early");}
            const bool hold=(cycle%2)!=0;ID3D11Texture2D* retained=hold?source.texture():nullptr;ID3D11Device* retained_device=hold?source.device():nullptr;
            if(retained){retained->AddRef();retained_device->AddRef();}
            struct Release {ID3D11Texture2D*& texture;ID3D11Device*& device;~Release(){if(texture){texture->Release();}if(device){device->Release();}}} release{retained,retained_device};
            const auto before=source.slots();const auto begin=mfs_d3d_lifetime_clock();source.close();const auto end=mfs_d3d_lifetime_clock();const auto after=source.slots();
            snapshot(rows,cycle,1);
            if(hold){require(read(4*cycle)[4]==0 && read(4*cycle+3)[4]==0,"explicit device/texture references notified early");}
            const auto release_time=mfs_d3d_lifetime_clock();if(retained){retained->Release();retained=nullptr;retained_device->Release();retained_device=nullptr;}
            const auto deadline=GetTickCount64()+2000;
            for(;;){bool done=true;for(uint32_t kind=0;kind<4;++kind){done&=read(4*cycle+kind)[4]==1;}if(done || GetTickCount64()>=deadline){break;}Sleep(1);}
            const auto final=mfs_d3d_lifetime_clock();snapshot(rows,cycle,2);
            calls<<cycle<<'\t'<<hold<<'\t'<<before<<'\t'<<after<<'\t'<<GetCurrentThreadId()<<'\t'<<begin<<'\t'<<end<<'\t'<<release_time<<'\t'<<final<<'\n';calls.flush();
            require(before==127 && after==0,"source did not clear every native slot");
            for(uint32_t kind=0;kind<4;++kind){require(read(4*cycle+kind)[4]==1,"native destruction/association notification did not complete");}
            fixture.close();
        }
        for(uint32_t cycle=0;cycle<20;++cycle){save(root,std::to_string(cycle)+"-held",held_images[cycle]);}
        std::cout<<"original DesktopDup lifetime cycles=20 native_objects=80 d3d_destruction_notifications=60 dxgi_private_owner_releases=20 held_texture_controls=10 held_device_controls=10 retained_cpu=20 extra_flush=0 full_P1_acceptance=0 passed\n";return 0;
    }catch(const std::exception& error){std::cerr<<"desktop-lifetime: "<<error.what()<<'\n';return 1;}
}
