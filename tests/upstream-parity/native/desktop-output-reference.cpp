// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Whole unchanged F class. Creation still uses the real default hardware.
#include "Screencap/DesktopDupScreencap.h"
#include "desktop-output-probe.h"
#include "d3d-lifetime.h"
#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
namespace {
void require(bool value,const char* message){if(!value)throw std::runtime_error(message);}
HRESULT WINAPI create_device(IDXGIAdapter* adapter,D3D_DRIVER_TYPE driver,HMODULE software,UINT flags,
    const D3D_FEATURE_LEVEL* levels,UINT count,UINT sdk,ID3D11Device** device,D3D_FEATURE_LEVEL* level,ID3D11DeviceContext** context) {
    require(!adapter && driver==D3D_DRIVER_TYPE_HARDWARE && !software && !flags && !levels && !count && sdk==D3D11_SDK_VERSION && device && !*device && !level && context && !*context,"source default device arguments differ");
    const auto code=::D3D11CreateDevice(adapter,driver,software,flags,levels,count,sdk,device,level,context);
    if(SUCCEEDED(code))require(!mfs_dd_output_created(*device,*context),"device observation failed");return code;
}
HRESULT WINAPI create_factory(REFIID iid,void** output) {
    require(iid==__uuidof(IDXGIFactory) && output && !*output,"source factory arguments differ");
    const auto code=::CreateDXGIFactory(iid,output);if(FAILED(code))return code;
    auto* native=static_cast<IDXGIFactory*>(*output);*output=nullptr;
    const auto wrapped=mfs_dd_output_factory(native,output);native->Release();return wrapped;
}
}
#define D3D11CreateDevice create_device
#define CreateDXGIFactory create_factory
#include MFS_UPSTREAM_DESKTOP_DUP_CPP
#undef D3D11CreateDevice
#undef CreateDXGIFactory
namespace {
struct Source : MaaNS::CtrlUnitNs::DesktopDupScreencap {
    using DesktopDupScreencap::DesktopDupScreencap;
    void close(){uninit();}
    unsigned slots()const{return unsigned(bool(d3d_device_))|(unsigned(bool(d3d_context_))<<1)|(unsigned(bool(dxgi_factory_))<<2)
        |(unsigned(bool(dxgi_adapter_))<<3)|(unsigned(bool(dxgi_output_))<<4)|(unsigned(bool(dxgi_dup_))<<5)|(unsigned(bool(readable_texture_))<<6);}
};
using Trace=std::array<uint64_t,MFS_DD_FIELDS>;
Trace trace(){Trace r{};require(!mfs_dd_output_trace(r.data(),r.size()),"output trace read failed");return r;}
void await_lifetimes(unsigned index){
    const auto deadline=GetTickCount64()+2000;
    for(;;){bool complete=true;for(unsigned kind=0;kind<2;++kind){std::array<uint64_t,12> r{};require(!mfs_d3d_lifetime_read(index*2+kind,r.data(),r.size()),"output lifetime wait read failed");complete&=r[4]==1;}
        if(complete || GetTickCount64()>=deadline)return;Sleep(1);}
}
void write(std::ostream& stream,unsigned cycle,unsigned control,unsigned phase,const Trace& r){stream<<cycle<<'\t'<<control<<'\t'<<phase;for(auto v:r)stream<<'\t'<<v;stream<<'\n';stream.flush();}
void lifetime(std::ostream& stream,unsigned cycle,unsigned control,unsigned phase,unsigned index){
    for(unsigned kind=0;kind<2;++kind){std::array<uint64_t,12> r{};require(!mfs_d3d_lifetime_read(index*2+kind,r.data(),r.size()),"output lifetime read failed");
        stream<<cycle<<'\t'<<control<<'\t'<<phase<<'\t'<<kind;for(auto v:r)stream<<'\t'<<v;stream<<'\n';stream.flush();
        require(r[0]==1 && r[3]==1 && r[4]==(phase?1u:0u),"output device/context lifetime phase differs");}
}
}
int wmain(int argc,wchar_t** argv){
    try {
        if(argc!=3)return 64;const std::wstring argument(argv[1]);
        require(argument.size()==1 && argument[0]>=L'0' && argument[0]<=L'3',"canonical primary-output mode 0..3 required");
        // The prior range check makes this character difference exactly 0..3.
        const auto mode=static_cast<unsigned>(argument[0]-L'0');const std::filesystem::path root(argv[2]);
        require(mode<4 && root.is_absolute() && std::filesystem::is_directory(root) && std::filesystem::is_empty(root),"new empty output root/mode required");
        require(mfs_d3d_lifetime_version()==1,"output lifetime observer version differs");
        std::array<uint32_t,6> widths{};require(!mfs_d3d_lifetime_widths(widths.data(),widths.size()) && widths==std::array<uint32_t,6>{8,8,8,4,4,4},"output observer ABI widths differ");
        std::ofstream abi(root/"abi.tsv",std::ios::binary),calls(root/"calls.tsv",std::ios::binary),traces(root/"trace.tsv",std::ios::binary),lives(root/"lifetimes.tsv",std::ios::binary);
        for(auto* out:{&abi,&calls,&traces,&lives})out->exceptions(std::ios::badbit|std::ios::failbit);
        abi<<1<<'\t'<<mode<<'\t'<<MFS_DD_FIELDS;for(auto width:widths)abi<<'\t'<<width;abi<<'\t'<<static_cast<uint32_t>(E_FAIL)<<'\t'<<static_cast<uint32_t>(E_NOINTERFACE)<<'\t'<<static_cast<uint32_t>(E_ABORT)<<'\n';abi.close();
        calls<<"cycle\tcontrol\tsuccess\tbefore_close_slots\tafter_close_slots\tsamples\tconversions\tsurface_code\tsurface_error\treturned\tclosed\tobserved\n";
        traces<<"cycle\tcontrol\tphase\tfields\n";lives<<"cycle\tcontrol\tphase\tkind\tfields\n";
        constexpr std::array<unsigned,4> failed_slots{7,15,15,31};
        for(unsigned cycle=0;cycle<20;++cycle)for(unsigned control=0;control<2;++control){
            const auto index=cycle*2+control;require(!mfs_dd_output_begin(mode,control,index),"output scope begin failed");
            Source source(nullptr);const auto success=source.screencap().has_value();const auto returned=mfs_d3d_lifetime_clock();const auto before=source.slots();
            write(traces,cycle,control,0,trace());lifetime(lives,cycle,control,0,index);
            source.close();const auto closed=mfs_d3d_lifetime_clock();write(traces,cycle,control,1,trace());await_lifetimes(index);lifetime(lives,cycle,control,1,index);
            const auto observed=mfs_d3d_lifetime_clock();
            const auto after=source.slots();require(!mfs_dd_output_end(),"output scope has residual proxies/violations");
            calls<<cycle<<'\t'<<control<<'\t'<<success<<'\t'<<before<<'\t'<<after<<"\t0\t0\t0\t-\t"<<returned<<'\t'<<closed<<'\t'<<observed<<'\n';calls.flush();
            require(!success && before==(control?31:failed_slots[mode]) && after==0,"source output failure/control slots differ");
        }
        std::cout<<"original DesktopDup primary_output mode="<<mode<<" cycles=20 public_rejections=40 primary_forwarding_controls=20 d3d_notifications=80 captures=0 full_P1_acceptance=0 passed\n";return 0;
    } catch(const std::exception& e){std::cerr<<"desktop output: "<<e.what()<<'\n';return 1;}
}
