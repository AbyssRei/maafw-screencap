// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// The complete unchanged F class; distinguish uninit from owning destruction.
#include "Screencap/FramePoolScreencap.h"
#include "d3d-lifetime.h"
#include "print-thread.h"
#include <roapi.h>
#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <vector>
namespace {
using Source=MaaNS::CtrlUnitNs::FramePoolScreencap;
template<class Tag,typename Tag::type Value> struct Access {friend typename Tag::type member(Tag){return Value;}};
#define FIELD(Tag,Type,Name) struct Tag {using type=Type Source::*;friend type member(Tag);};template struct Access<Tag,&Source::Name>
FIELD(Device,winrt::com_ptr<ID3D11Device>,d3d_device_);
FIELD(Context,winrt::com_ptr<ID3D11DeviceContext>,d3d_context_);
FIELD(Swap,winrt::com_ptr<IDXGISwapChain>,dxgi_swap_chain_);
FIELD(Texture,winrt::com_ptr<ID3D11Texture2D>,readable_texture_);
FIELD(Item,winrt::Windows::Graphics::Capture::GraphicsCaptureItem,cap_item_);
FIELD(Pool,winrt::Windows::Graphics::Capture::Direct3D11CaptureFramePool,cap_frame_pool_);
FIELD(Session,winrt::Windows::Graphics::Capture::GraphicsCaptureSession,cap_session_);
FIELD(Cache,cv::Mat,cached_image_);
#undef FIELD
struct Stop {using type=void(Source::*)();friend type member(Stop);};template struct Access<Stop,&Source::uninit>;
constexpr std::array<uint32_t,3> kinds{0,1,3};
void require(bool b,const char* text){if(!b){throw std::runtime_error(text);}}
void status(int32_t code){require(code==0,"native lifetime observer failed");}
unsigned slots(Source& s){return unsigned(bool(s.*member(Device{})))|(unsigned(bool(s.*member(Context{})))<<1)
    |(unsigned(bool(s.*member(Swap{})))<<2)|(unsigned(bool(s.*member(Texture{})))<<3)
    |(unsigned(bool(s.*member(Item{})))<<4)|(unsigned(bool(s.*member(Pool{})))<<5)|(unsigned(bool(s.*member(Session{})))<<6);}
std::array<uint64_t,12> read(uint32_t index){std::array<uint64_t,12> r{};status(mfs_d3d_lifetime_read(index,r.data(),r.size()));return r;}
void snapshot(std::ostream& rows,uint32_t cycle,uint32_t phase){
    for(uint32_t i=0;i<3;++i){const auto r=read(cycle*3+i);rows<<cycle<<'\t'<<phase<<'\t'<<kinds[i];for(auto value:r){rows<<'\t'<<value;}rows<<'\n';}rows.flush();
}
void save(const std::filesystem::path& root,const std::string& name,const cv::Mat& image){
    require(image.type()==CV_8UC3 && image.rows==64 && image.cols==96,"target CPU layout differs");
    std::ofstream pixels(root/(name+".bgr"),std::ios::binary);pixels.exceptions(std::ios::badbit|std::ios::failbit);
    for(int y=0;y<64;++y){pixels.write(reinterpret_cast<const char*>(image.ptr(y)),288);}pixels.close();
    const auto marker=image.at<cv::Vec3b>(0,0);require(marker==cv::Vec3b(78,95,96) || marker==cv::Vec3b(26,43,60),"target marker differs");
    for(int y=0;y<64;++y){for(int x=0;x<96;++x){const auto expected=x<16 && y<16?marker:x<32?cv::Vec3b(31,83,167):y<32?cv::Vec3b(185,77,39):cv::Vec3b(59,226,129);require(image.at<cv::Vec3b>(y,x)==expected,"target pixels differ");}}
}
struct Window {
    MfsPrintFixture value{};
    void close(){if(value.owner){require(mfs_print_thread_close(&value.owner)==0,"window close failed");}}
    ~Window(){if(value.owner && mfs_print_thread_close(&value.owner)){std::terminate();}}
};
}
int wmain(int argc,wchar_t** argv){
    if(argc!=2){return 64;}const std::filesystem::path root(argv[1]);
    if(!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root)){return 64;}
    try {
        winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED));struct Apartment {~Apartment(){RoUninitialize();}} apartment;
        require(mfs_d3d_lifetime_version()==1,"observer version differs");
        std::array<uint32_t,6> widths{};status(mfs_d3d_lifetime_widths(widths.data(),widths.size()));require(widths==std::array<uint32_t,6>{8,8,8,4,4,4},"native ABI widths differ");
        std::ofstream abi(root/"abi.tsv",std::ios::binary);abi.exceptions(std::ios::badbit|std::ios::failbit);abi<<1;for(auto w:widths){abi<<'\t'<<w;}abi<<'\n';abi.close();
        std::ofstream rows(root/"lifetimes.tsv",std::ios::binary),calls(root/"calls.tsv",std::ios::binary);
        rows.exceptions(std::ios::badbit|std::ios::failbit);calls.exceptions(std::ios::badbit|std::ios::failbit);
        rows<<"cycle\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr\n";
        calls<<"cycle\theld\tbefore_slots\tstopped_slots\tcaller\tbegin_stop\tend_stop\tbegin_drop\tend_drop\trelease_qpc\tfinal_qpc\timage\tcache\tstopped_cache\n";
        std::vector<cv::Mat> held_images;held_images.reserve(20);
        for(uint32_t cycle=0;cycle<20;++cycle){
            Window window;require(!mfs_print_thread_fixture(96,64,0,0,0,0,MFS_RENDER_DISABLE_TRANSITIONS|MFS_RENDER_TOPMOST,&window.value),"owned scene failed");
            const std::array<int64_t,6> request{1,WS_POPUP|WS_VISIBLE,100,100,96,64};std::array<int64_t,31> geometry{};
            require(!mfs_print_thread_geometry(window.value.owner,request.data(),request.size(),geometry.data(),geometry.size())
                && geometry[14]==96 && geometry[15]==64 && !mfs_desktop_fixture_inside_updates(window.value.owner),"owned scene geometry/update failed");
            auto source=std::make_unique<Source>(static_cast<HWND>(window.value.hwnd));auto image=source->screencap();require(image.has_value(),"source returned no frame");
            save(root,std::to_string(cycle)+"-current",*image);held_images.push_back(*image);
            const auto result_address=reinterpret_cast<uintptr_t>(image->data),cache_address=reinterpret_cast<uintptr_t>(((*source).*member(Cache{})).data);
            require(result_address && cache_address && result_address!=cache_address,"source result/cache are not independent CPU allocations");
            const std::array<void*,3> objects{((*source).*member(Device{})).get(),((*source).*member(Context{})).get(),((*source).*member(Texture{})).get()};
            for(uint32_t i=0;i<3;++i){status(mfs_d3d_lifetime_attach(objects[i],kinds[i],3*cycle+i));}
            snapshot(rows,cycle,0);for(uint32_t i=0;i<3;++i){require(read(3*cycle+i)[4]==0,"live native object notified early");}
            const bool hold=cycle%2!=0;
            auto retained_device=hold?((*source).*member(Device{})):winrt::com_ptr<ID3D11Device>{};
            auto retained_texture=hold?((*source).*member(Texture{})):winrt::com_ptr<ID3D11Texture2D>{};
            const auto before=slots(*source);const auto begin=mfs_d3d_lifetime_clock();((*source).*member(Stop{}))();const auto end=mfs_d3d_lifetime_clock();
            const auto stopped=slots(*source);const auto stopped_cache=reinterpret_cast<uintptr_t>(((*source).*member(Cache{})).data);
            snapshot(rows,cycle,1);require(before==127 && stopped==7 && stopped_cache==cache_address,"source uninit retention differs");
            require(read(3*cycle)[4]==0 && read(3*cycle+1)[4]==0 && (!hold || read(3*cycle+2)[4]==0),"source owned/explicit references notified before destruction");
            const auto begin_drop=mfs_d3d_lifetime_clock();source.reset();const auto end_drop=mfs_d3d_lifetime_clock();
            if(!hold){const auto deadline=GetTickCount64()+2000;for(;;){bool done=true;for(uint32_t i=0;i<3;++i){done&=read(3*cycle+i)[4]==1;}if(done || GetTickCount64()>=deadline){break;}Sleep(1);}}
            snapshot(rows,cycle,2);if(hold){require(read(3*cycle)[4]==0 && read(3*cycle+2)[4]==0,"explicit references notified before release");}
            else {for(uint32_t i=0;i<3;++i){require(read(3*cycle+i)[4]==1,"owning destruction notification exceeded observation budget");}}
            const auto release=mfs_d3d_lifetime_clock();retained_texture=nullptr;retained_device=nullptr;
            const auto deadline=GetTickCount64()+2000;for(;;){bool done=true;for(uint32_t i=0;i<3;++i){done&=read(3*cycle+i)[4]==1;}if(done || GetTickCount64()>=deadline){break;}Sleep(1);}
            const auto final=mfs_d3d_lifetime_clock();snapshot(rows,cycle,3);
            calls<<cycle<<'\t'<<hold<<'\t'<<before<<'\t'<<stopped<<'\t'<<GetCurrentThreadId()<<'\t'<<begin<<'\t'<<end<<'\t'<<begin_drop<<'\t'<<end_drop<<'\t'<<release<<'\t'<<final<<'\t'<<result_address<<'\t'<<cache_address<<'\t'<<stopped_cache<<'\n';calls.flush();
            for(uint32_t i=0;i<3;++i){require(read(3*cycle+i)[4]==1,"native destruction notification did not complete");}window.close();
        }
        for(uint32_t cycle=0;cycle<20;++cycle){save(root,std::to_string(cycle)+"-held",held_images[cycle]);}
        std::cout<<"original FramePool lifetime cycles=20 public_calls=20 d3d_destruction_notifications=60 explicit_reference_pairs=10 uninit_gpu_slots=7 retained_cpu=20 extra_flush=0 full_P1_acceptance=0 passed\n";return 0;
    }catch(const winrt::hresult_error& error){std::cerr<<"framepool-lifetime HRESULT="<<error.code().value<<'\n';}
    catch(const std::exception& error){std::cerr<<"framepool-lifetime: "<<error.what()<<'\n';}return 1;
}
