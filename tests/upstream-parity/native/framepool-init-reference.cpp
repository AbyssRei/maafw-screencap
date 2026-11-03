// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Unchanged F class; hidden-window native initialization failures, no frames.
#include "Screencap/FramePoolScreencap.h"
#include "framepool-init-fixture.h"
#include <roapi.h>
#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>

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
#undef FIELD
void require(bool value,const char* message){if(!value){throw std::runtime_error(message);}}
unsigned slots(Source& s){return unsigned(bool(s.*member(Device{})))|(unsigned(bool(s.*member(Context{})))<<1)
    |(unsigned(bool(s.*member(Swap{})))<<2)|(unsigned(bool(s.*member(Texture{})))<<3)
    |(unsigned(bool(s.*member(Item{})))<<4)|(unsigned(bool(s.*member(Pool{})))<<5)|(unsigned(bool(s.*member(Session{})))<<6);}
uint64_t qpc(){LARGE_INTEGER value{};require(QueryPerformanceCounter(&value)!=FALSE,"QPC failed");return static_cast<uint64_t>(value.QuadPart);}
struct Window {
    void* value=nullptr;
    Window(){winrt::check_hresult(mfs_framepool_init_window_create(&value));}
    void close(){if(value){winrt::check_hresult(mfs_framepool_init_window_destroy(&value));}}
    ~Window(){if(value && FAILED(mfs_framepool_init_window_destroy(&value))){std::terminate();}}
};
}
int wmain(int argc,wchar_t** argv){
    if(argc!=2){return 64;}
    const std::filesystem::path root(argv[1]);
    if(!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root)){return 64;}
    try {
        require(mfs_framepool_init_version()==1,"fixture version differs");
        std::array<uint32_t,6> widths{};winrt::check_hresult(mfs_framepool_init_widths(widths.data(),widths.size()));
        require(widths==std::array<uint32_t,6>{8,8,8,4,4,4},"native scalar widths differ");
        {std::ofstream abi(root/"abi.tsv",std::ios::binary);abi.exceptions(std::ios::badbit|std::ios::failbit);abi<<1;for(auto width:widths){abi<<'\t'<<width;}abi<<'\n';}
        winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED));
        struct Initialization {~Initialization(){RoUninitialize();}} initialization;
        std::ofstream rows(root/"calls.tsv",std::ios::binary),lifetimes(root/"lifetimes.tsv",std::ios::binary);
        rows.exceptions(std::ios::badbit|std::ios::failbit);lifetimes.exceptions(std::ios::badbit|std::ios::failbit);
        rows<<"cycle\tcall\tvalid\tvisible\tbefore_slots\tafter_slots\tno_image\tprevious_final\tcaller\twindow_thread\tbegin_qpc\tend_qpc\n";
        lifetimes<<"index\tattached\tfinal\tattach_thread\tfinal_thread\tattach_qpc\tfinal_qpc\n";
        for(unsigned cycle=0;cycle<20;++cycle){
            Window window;const auto target=static_cast<HWND>(window.value);
            {
                Source source(target);
                for(unsigned call=0;call<4;++call){
                    if(call==2){window.close();}
                    std::array<uint64_t,4> info{};winrt::check_hresult(mfs_framepool_init_window_info(target,info.data(),info.size()));
                    require(info[0] && info[2]==unsigned(call<2) && !info[3] && info[1]==(call<2?info[0]:0),"hidden/destroyed owner premise differs");
                    const auto before=slots(source);const auto begin=qpc();auto image=source.screencap();const auto end=qpc();
                    const auto after=slots(source);uint64_t previous=0;
                    if(call==1 || call==2){std::array<uint64_t,6> values{};winrt::check_hresult(mfs_framepool_init_record(2*cycle+call-1,values.data(),values.size()));previous=values[1];}
                    rows<<cycle<<'\t'<<call<<'\t'<<info[2]<<'\t'<<info[3]<<'\t'<<before<<'\t'<<after<<'\t'<<!image<<'\t'<<previous<<'\t'<<info[0]<<'\t'<<info[1]<<'\t'<<begin<<'\t'<<end<<'\n';rows.flush();
                    require(!image && before==(call==0 || call==3 ? 0u:7u) && after==(call<2?7u:0u),"source partial initialization state differs");
                    require(previous==unsigned(call==1 || call==2),"old device private-data owner survived replacement");
                    if(call<2){winrt::check_hresult(mfs_framepool_init_attach((source.*member(Device{})).get(),2*cycle+call));}
                }
            }
        }
        for(uint32_t i=0;i<40;++i){
            std::array<uint64_t,6> values{};winrt::check_hresult(mfs_framepool_init_record(i,values.data(),values.size()));
            require(values[0]==1 && values[1]==1 && values[2] && values[3] && values[4] && values[5]>=values[4],"native final Release record differs");
            lifetimes<<i;for(auto value:values){lifetimes<<'\t'<<value;}lifetimes<<'\n';
        }
        std::cout<<"original FramePool initialization cycles=20 calls=80 hidden_rejections=40 destroyed_rejections=40 final_device_notifications=40 native_captures=0 full_P1_acceptance=0 passed\n";
        return 0;
    }catch(const winrt::hresult_error& error){std::cerr<<"init HRESULT="<<error.code().value<<'\n';}
     catch(const std::exception& error){std::cerr<<"init failure="<<error.what()<<'\n';}
    return 1;
}
