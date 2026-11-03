// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Only two native creation boundaries are controlled; the whole F body is included unchanged.
#include "Screencap/DesktopDupScreencap.h"
#include "d3d-lifetime.h"
#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
namespace {
using Life=std::array<uint64_t,12>;
struct Trace {
    unsigned mode{},first_index{},order{},device_calls{},native_devices{},factory_calls{},native_factories{};
    HRESULT device_result{},factory_result{};DWORD caller{};bool violation{};std::array<Life,2> live{};
};
thread_local Trace* active=nullptr;
constexpr std::array<HRESULT,4> codes{E_FAIL,E_OUTOFMEMORY,E_FAIL,E_NOINTERFACE};
Life read(unsigned index){Life value{};if(mfs_d3d_lifetime_read(index,value.data(),value.size()))throw std::runtime_error("initialization observer read failed");return value;}
HRESULT WINAPI probe_device(IDXGIAdapter* adapter,D3D_DRIVER_TYPE driver,HMODULE software,UINT flags,
    const D3D_FEATURE_LEVEL* levels,UINT level_count,UINT sdk,ID3D11Device** device,D3D_FEATURE_LEVEL* level,ID3D11DeviceContext** context){
    if(!active)return ::D3D11CreateDevice(adapter,driver,software,flags,levels,level_count,sdk,device,level,context);
    auto& t=*active;t.caller=GetCurrentThreadId();++t.device_calls;t.violation|=t.order!=0;t.order=1;
    t.violation|=adapter || driver!=D3D_DRIVER_TYPE_HARDWARE || software || flags || levels || level_count || sdk!=D3D11_SDK_VERSION || !device || level || !context;
    t.violation|=device && *device || context && *context;
    if(t.mode<2){t.device_result=codes[t.mode];return t.device_result;}
    ++t.native_devices;t.device_result=::D3D11CreateDevice(adapter,driver,software,flags,levels,level_count,sdk,device,level,context);
    if(SUCCEEDED(t.device_result) && t.mode<4){
        std::array<void*,2> values{*device,*context};
        for(unsigned kind=0;kind<2;++kind){t.violation|=mfs_d3d_lifetime_attach(values[kind],kind,t.first_index+kind)!=0;t.live[kind]=read(t.first_index+kind);}
    }
    return t.device_result;
}
HRESULT WINAPI probe_factory(REFIID iid,void** output){
    if(!active)return ::CreateDXGIFactory(iid,output);
    auto& t=*active;t.violation|=t.caller!=GetCurrentThreadId() || t.order!=1 || t.mode<2 || iid!=__uuidof(IDXGIFactory) || !output;
    t.violation|=output && *output;t.order=12;++t.factory_calls;
    if(t.mode<4){t.factory_result=codes[t.mode];return t.factory_result;}
    ++t.native_factories;t.factory_result=::CreateDXGIFactory(iid,output);return t.factory_result;
}
}
#define D3D11CreateDevice probe_device
#define CreateDXGIFactory probe_factory
#include MFS_UPSTREAM_DESKTOP_DUP_CPP
#undef D3D11CreateDevice
#undef CreateDXGIFactory
namespace {
void require(bool value,const char* text){if(!value)throw std::runtime_error(text);}
struct Source : MaaNS::CtrlUnitNs::DesktopDupScreencap {
    using DesktopDupScreencap::DesktopDupScreencap;
    bool initialize(){return init();}
    void close(){uninit();}
    unsigned slots()const{return unsigned(bool(d3d_device_))|(unsigned(bool(d3d_context_))<<1)|(unsigned(bool(dxgi_factory_))<<2)
        |(unsigned(bool(dxgi_adapter_))<<3)|(unsigned(bool(dxgi_output_))<<4)|(unsigned(bool(dxgi_dup_))<<5)|(unsigned(bool(readable_texture_))<<6);}
};
struct Scope {explicit Scope(Trace& trace){require(!active,"nested initialization observer");active=&trace;}~Scope(){active=nullptr;}};
void life(std::ostream& out,unsigned cycle,unsigned mode,unsigned phase,const std::array<Life,2>& rows){
    for(unsigned kind=0;kind<2;++kind){out<<cycle<<'\t'<<mode<<'\t'<<phase<<'\t'<<kind;for(auto v:rows[kind])out<<'\t'<<v;out<<'\n';}out.flush();
}
}
int wmain(int argc,wchar_t** argv){
    if(argc!=2)return 64;const std::filesystem::path root(argv[1]);
    if(!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root))return 64;
    try{
        require(mfs_d3d_lifetime_version()==1,"initialization observer version differs");
        std::array<uint32_t,6> widths{};require(!mfs_d3d_lifetime_widths(widths.data(),widths.size()) && widths==std::array<uint32_t,6>{8,8,8,4,4,4},"initialization ABI widths differ");
        std::ofstream abi(root/"abi.tsv",std::ios::binary),calls(root/"calls.tsv",std::ios::binary),rows(root/"lifetimes.tsv",std::ios::binary);
        for(auto* out:{&abi,&calls,&rows})out->exceptions(std::ios::badbit|std::ios::failbit);
        abi<<2;for(auto width:widths)abi<<'\t'<<width;for(auto code:codes)abi<<'\t'<<static_cast<uint32_t>(code);abi<<'\n';abi.close();
        calls<<"cycle\tmode\tcontrol\tsuccess\torder\tdevice_calls\tnative_devices\tfactory_calls\tnative_factories\tinjected_code\tsurface_code\tbefore_close_slots\tafter_close_slots\tconversions\tsamples\tcaller\tbegin\treturned\tobserved\tdevice_result\tfactory_result\tviolation\n";
        rows<<"cycle\tmode\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr\n";
        for(unsigned cycle=0;cycle<20;++cycle){for(unsigned mode=0;mode<5;++mode){
            Trace trace;trace.mode=mode;trace.first_index=mode>=2 && mode<4?(cycle*2+mode-2)*2:0;Source source(nullptr);
            require(source.slots()==0,"new source has native slots");bool success=false;const auto begin=mfs_d3d_lifetime_clock();
            {Scope scope(trace);success=mode==4?source.initialize():source.screencap().has_value();}
            const auto returned=mfs_d3d_lifetime_clock();const auto before=source.slots();
            std::array<Life,2> at_return{},final{};
            if(mode>=2 && mode<4){
                for(unsigned kind=0;kind<2;++kind)at_return[kind]=read(trace.first_index+kind);
                life(rows,cycle,mode,0,trace.live);life(rows,cycle,mode,1,at_return);
                const auto deadline=GetTickCount64()+2000;
                for(;;){for(unsigned kind=0;kind<2;++kind)final[kind]=read(trace.first_index+kind);
                    if((final[0][4]==1 && final[1][4]==1) || GetTickCount64()>=deadline)break;Sleep(1);}
                life(rows,cycle,mode,2,final);
            }
            const auto observed=mfs_d3d_lifetime_clock();source.close();
            calls<<cycle<<'\t'<<mode<<'\t'<<(mode==4)<<'\t'<<success<<'\t'<<trace.order<<'\t'<<trace.device_calls<<'\t'<<trace.native_devices<<'\t'<<trace.factory_calls<<'\t'<<trace.native_factories<<'\t'<<(mode<4?static_cast<uint32_t>(codes[mode]):0)<<"\t0\t"<<before<<'\t'<<source.slots()<<"\t0\t0\t"<<trace.caller<<'\t'<<begin<<'\t'<<returned<<'\t'<<observed<<'\t'<<static_cast<uint32_t>(trace.device_result)<<'\t'<<static_cast<uint32_t>(trace.factory_result)<<'\t'<<trace.violation<<'\n';calls.flush();
            require(!trace.violation && success==(mode==4) && before==(mode==4?7:0) && source.slots()==0,"source initialization boundary/result/slots differ");
            if(mode>=2 && mode<4)for(unsigned kind=0;kind<2;++kind)require(trace.live[kind][4]==0 && final[kind][4]==1,"partial initialization object not released");
        }}
        std::cout<<"original DesktopDup initialization cycles=20 public_rejections=80 direct_init_controls=20 native_devices=60 native_factories=20 d3d_notifications=80 captures=0 full_P1_acceptance=0 passed\n";return 0;
    }catch(const std::exception& e){std::cerr<<"desktop initialization: "<<e.what()<<'\n';return 1;}
}
