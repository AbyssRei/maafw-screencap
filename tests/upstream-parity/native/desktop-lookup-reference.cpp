// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Whole unchanged F class; only device/factory creation symbols are observed.
#include "Screencap/DesktopDupScreencap.h"
#include "desktop-lookup-probe.h"
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
    if(SUCCEEDED(code))require(!mfs_dd_lookup_created(*device,*context),"lookup device observation failed");return code;
}
HRESULT WINAPI create_factory(REFIID iid,void** output) {
    require(iid==__uuidof(IDXGIFactory) && output && !*output,"source factory arguments differ");
    const auto code=::CreateDXGIFactory(iid,output);if(FAILED(code))return code;
    auto* native=static_cast<IDXGIFactory*>(*output);*output=nullptr;
    const auto wrapped=mfs_dd_lookup_factory(native,output);native->Release();return wrapped;
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
    uint64_t monitor()const{return reinterpret_cast<uintptr_t>(current_monitor_);}
    unsigned slots()const{return unsigned(bool(d3d_device_))|(unsigned(bool(d3d_context_))<<1)|(unsigned(bool(dxgi_factory_))<<2)
        |(unsigned(bool(dxgi_adapter_))<<3)|(unsigned(bool(dxgi_output_))<<4)|(unsigned(bool(dxgi_dup_))<<5)|(unsigned(bool(readable_texture_))<<6);}
};
using Trace=std::array<uint64_t,MFS_LOOKUP_FIELDS>;
Trace trace(){Trace r{};require(!mfs_dd_lookup_trace(r.data(),r.size()),"lookup trace read failed");return r;}
void write(std::ostream& stream,unsigned cycle,unsigned control,unsigned phase,const Trace& r){stream<<cycle<<'\t'<<control<<'\t'<<phase;for(auto v:r)stream<<'\t'<<v;stream<<'\n';stream.flush();}
void lifetime(std::ostream& stream,unsigned cycle,unsigned control,unsigned phase,unsigned index){
    for(unsigned kind=0;kind<2;++kind){std::array<uint64_t,12> r{};require(!mfs_d3d_lifetime_read(index*2+kind,r.data(),r.size()),"lookup lifetime read failed");
        stream<<cycle<<'\t'<<control<<'\t'<<phase<<'\t'<<kind;for(auto v:r)stream<<'\t'<<v;stream<<'\n';stream.flush();
        require(r[0]==1 && r[3]==1 && r[4]==(phase?1u:0u),"lookup D3D lifetime phase differs");}
}
void await_lifetimes(unsigned index){
    const auto deadline=GetTickCount64()+2000;
    for(;;){bool done=true;for(unsigned kind=0;kind<2;++kind){std::array<uint64_t,12> r{};require(!mfs_d3d_lifetime_read(index*2+kind,r.data(),r.size()),"lookup lifetime wait failed");done&=r[4]==1;}
        if(done || GetTickCount64()>=deadline)return;Sleep(1);}
}
void target_row(std::ostream& out,unsigned phase,const std::array<uint64_t,7>& values){out<<phase;for(auto v:values)out<<'\t'<<v;out<<'\n';out.flush();}
}
int wmain(int argc,wchar_t** argv){
    try {
        if(argc!=3)return 64;const std::wstring argument(argv[1]);
        require(argument.size()==1 && argument[0]>=L'0' && argument[0]<=L'4',"canonical lookup mode 0..4 required");
        const auto mode=static_cast<unsigned>(argument[0]-L'0');const std::filesystem::path root(argv[2]);
        require(root.is_absolute() && std::filesystem::is_directory(root) && std::filesystem::is_empty(root),"new empty lookup root required");
        std::array<uint32_t,6> widths{};require(mfs_d3d_lifetime_version()==1 && !mfs_d3d_lifetime_widths(widths.data(),widths.size()) && widths==std::array<uint32_t,6>{8,8,8,4,4,4},"lookup observer widths differ");
        std::ofstream abi(root/"abi.tsv",std::ios::binary),calls(root/"calls.tsv",std::ios::binary),traces(root/"trace.tsv",std::ios::binary),lives(root/"lifetimes.tsv",std::ios::binary),targets(root/"target.tsv",std::ios::binary);
        for(auto* out:{&abi,&calls,&traces,&lives,&targets})out->exceptions(std::ios::badbit|std::ios::failbit);
        abi<<1<<'\t'<<mode<<'\t'<<MFS_LOOKUP_FIELDS;for(auto width:widths)abi<<'\t'<<width;for(auto code:{E_FAIL,E_NOINTERFACE,E_ABORT,DXGI_ERROR_NOT_FOUND})abi<<'\t'<<static_cast<uint32_t>(code);abi<<'\n';abi.close();
        calls<<"cycle\tcontrol\tsuccess\tbefore_slots\tafter_slots\tbefore_monitor\tafter_monitor\tsamples\tconversions\tsurface_code\tsurface_error\treturned\tclosed\tobserved\n";
        traces<<"cycle\tcontrol\tphase\tfields\n";lives<<"cycle\tcontrol\tphase\tkind\tfields\n";
        targets<<"phase\thwnd\tmonitor\tcaller\towner\tvisible\tvalid\tresolved\n";
        std::array<uint64_t,7> target{};require(!mfs_dd_lookup_target_create(target.data(),target.size()),"hidden lookup target unavailable");target_row(targets,0,target);
        for(unsigned cycle=0;cycle<20;++cycle)for(unsigned control=0;control<2;++control){
            const auto index=cycle*2+control;require(!mfs_dd_lookup_begin(mode,control,index),"lookup scope begin failed");
            Source source(reinterpret_cast<HWND>(static_cast<uintptr_t>(target[0])));const auto success=source.screencap().has_value();
            const auto returned=mfs_d3d_lifetime_clock(),before_monitor=source.monitor();const auto before=source.slots();
            write(traces,cycle,control,0,trace());lifetime(lives,cycle,control,0,index);
            source.close();const auto closed=mfs_d3d_lifetime_clock();write(traces,cycle,control,1,trace());await_lifetimes(index);lifetime(lives,cycle,control,1,index);
            const auto observed=mfs_d3d_lifetime_clock(),after_monitor=source.monitor();const auto after=source.slots();
            require(!mfs_dd_lookup_end(),"lookup scope residual owners/violations");
            calls<<cycle<<'\t'<<control<<'\t'<<success<<'\t'<<before<<'\t'<<after<<'\t'<<before_monitor<<'\t'<<after_monitor<<"\t0\t0\t0\t-\t"<<returned<<'\t'<<closed<<'\t'<<observed<<'\n';calls.flush();
            require(!success && before==31 && after==0 && before_monitor==(control?target[1]:0) && after_monitor==before_monitor,"lookup partial ownership/monitor differs");
        }
        require(!mfs_dd_lookup_target_close(target.data(),target.size()),"hidden target close failed");target_row(targets,1,target);
        std::cout<<"original DesktopDup lookup mode="<<mode<<" cycles=20 public_rejections=40 fallbacks=20 matching_controls=20 d3d_notifications=80 captures=0 full_P1_acceptance=0 passed\n";return 0;
    }catch(const std::exception& error){std::cerr<<"desktop lookup: "<<error.what()<<'\n';return 1;}
}
