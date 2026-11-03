// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Scoped lookup COM facades; explicit tail bounds and no driver duplication.
#define MFS_DD_LOOKUP_EXPORTS
#include "desktop-lookup-probe.h"
#include "d3d-lifetime.h"
#include <windows.h>
#include <dxgi1_2.h>
#include <wrl/client.h>
#include <array>
#include <algorithm>
#include <atomic>
#include <new>

namespace {
using Microsoft::WRL::ComPtr;
struct State { bool used=false; uint32_t index=0; unsigned zero_adapters=0; std::array<uint64_t,MFS_LOOKUP_FIELDS> v{}; };
std::array<State,40> states;
thread_local State* active=nullptr;
HWND target=nullptr;HMONITOR selected_monitor=nullptr;DWORD target_caller=0;
constexpr wchar_t target_class[]=L"mfs-dd-lookup-hidden-target";
bool fallback(const State& s) noexcept{return s.v[MFS_LOOKUP_FALLBACKS]!=0;}
void caller(State& state) noexcept { state.v[MFS_LOOKUP_VIOLATION]|=state.v[MFS_LOOKUP_CALLER]!=GetCurrentThreadId(); }
bool fails(State& s,unsigned mode) noexcept { return !s.v[MFS_LOOKUP_CONTROL] && !fallback(s) && s.v[MFS_LOOKUP_MODE]==mode; }
HRESULT rejected(State& s,unsigned mode) noexcept {
    const HRESULT code=mode==3?E_NOINTERFACE:E_FAIL;
    ++s.v[MFS_LOOKUP_FAULTS];s.v[MFS_LOOKUP_FAULT_HRESULT]=static_cast<uint32_t>(code);return code;
}
template<class I,class D,unsigned Kind> struct Owner : I {
    State& state;std::atomic<ULONG> refs{1};
    explicit Owner(State& s) noexcept:state(s) {caller(s);++s.v[MFS_LOOKUP_FACTORY_CREATED+2*Kind];}
    ~Owner() {caller(state);++state.v[MFS_LOOKUP_FACTORY_DESTROYED+2*Kind];}
    ULONG STDMETHODCALLTYPE AddRef() noexcept override {caller(state);return ++refs;}
    ULONG STDMETHODCALLTYPE Release() noexcept override {caller(state);auto count=--refs;if(!count)delete static_cast<D*>(this);return count;}
};
// Other IDXGIObject calls retain their ordinary native implementation. Only
// the explicitly exposed IUnknown/base/selected interfaces have facade identity.
#define OBJECT_FORWARD \
    HRESULT STDMETHODCALLTYPE SetPrivateData(REFGUID g,UINT n,const void* p) noexcept override{return raw->SetPrivateData(g,n,p);} \
    HRESULT STDMETHODCALLTYPE SetPrivateDataInterface(REFGUID g,const IUnknown* p) noexcept override{return raw->SetPrivateDataInterface(g,p);} \
    HRESULT STDMETHODCALLTYPE GetPrivateData(REFGUID g,UINT* n,void* p) noexcept override{return raw->GetPrivateData(g,n,p);} \
    HRESULT STDMETHODCALLTYPE GetParent(REFIID i,void** p) noexcept override{return raw->GetParent(i,p);}

struct Output final : Owner<IDXGIOutput1,Output,2> {
    ComPtr<IDXGIOutput> raw;ComPtr<IDXGIOutput1> advanced;
    Output(State& s,IDXGIOutput* value) noexcept:Owner(s),raw(value) {s.v[MFS_LOOKUP_OUTPUT_POINTER]=reinterpret_cast<uintptr_t>(static_cast<IDXGIOutput1*>(this));}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) noexcept override {
        caller(state);if(!out)return E_POINTER;*out=nullptr;
        if(iid==__uuidof(IDXGIOutput1)) {
            ++state.v[MFS_LOOKUP_QUERY];
            if(fails(state,3))return rejected(state,3);
            ++state.v[MFS_LOOKUP_NATIVE_QUERY];const auto code=raw.As(&advanced);if(FAILED(code))return code;
        } else if(iid!=__uuidof(IUnknown) && iid!=__uuidof(IDXGIObject) && iid!=__uuidof(IDXGIOutput))return E_NOINTERFACE;
        *out=static_cast<IDXGIOutput1*>(this);AddRef();return S_OK;
    }
    OBJECT_FORWARD
    HRESULT STDMETHODCALLTYPE GetDesc(DXGI_OUTPUT_DESC* p) noexcept override{
        caller(state);++state.v[MFS_LOOKUP_DESC];if(!p)return E_POINTER;
        if(fails(state,2))return rejected(state,2);
        ++state.v[MFS_LOOKUP_NATIVE_DESC];const auto code=raw->GetDesc(p);
        if(SUCCEEDED(code) && fails(state,4)){p->Monitor=nullptr;++state.v[MFS_LOOKUP_FAULTS];}
        return code;
    }
    HRESULT STDMETHODCALLTYPE GetDisplayModeList(DXGI_FORMAT f,UINT a,UINT* n,DXGI_MODE_DESC* p) noexcept override{return raw->GetDisplayModeList(f,a,n,p);}
    HRESULT STDMETHODCALLTYPE FindClosestMatchingMode(const DXGI_MODE_DESC* a,DXGI_MODE_DESC* b,IUnknown* d) noexcept override{return raw->FindClosestMatchingMode(a,b,d);}
    HRESULT STDMETHODCALLTYPE WaitForVBlank() noexcept override{return raw->WaitForVBlank();}
    HRESULT STDMETHODCALLTYPE TakeOwnership(IUnknown* d,BOOL e) noexcept override{return raw->TakeOwnership(d,e);}
    void STDMETHODCALLTYPE ReleaseOwnership() noexcept override{raw->ReleaseOwnership();}
    HRESULT STDMETHODCALLTYPE GetGammaControlCapabilities(DXGI_GAMMA_CONTROL_CAPABILITIES* p) noexcept override{return raw->GetGammaControlCapabilities(p);}
    HRESULT STDMETHODCALLTYPE SetGammaControl(const DXGI_GAMMA_CONTROL* p) noexcept override{return raw->SetGammaControl(p);}
    HRESULT STDMETHODCALLTYPE GetGammaControl(DXGI_GAMMA_CONTROL* p) noexcept override{return raw->GetGammaControl(p);}
    HRESULT STDMETHODCALLTYPE SetDisplaySurface(IDXGISurface* p) noexcept override{return raw->SetDisplaySurface(p);}
    HRESULT STDMETHODCALLTYPE GetDisplaySurfaceData(IDXGISurface* p) noexcept override{return raw->GetDisplaySurfaceData(p);}
    HRESULT STDMETHODCALLTYPE GetFrameStatistics(DXGI_FRAME_STATISTICS* p) noexcept override{return raw->GetFrameStatistics(p);}
    HRESULT STDMETHODCALLTYPE GetDisplayModeList1(DXGI_FORMAT f,UINT a,UINT* n,DXGI_MODE_DESC1* p) noexcept override{return advanced?advanced->GetDisplayModeList1(f,a,n,p):E_NOINTERFACE;}
    HRESULT STDMETHODCALLTYPE FindClosestMatchingMode1(const DXGI_MODE_DESC1* a,DXGI_MODE_DESC1* b,IUnknown* d) noexcept override{return advanced?advanced->FindClosestMatchingMode1(a,b,d):E_NOINTERFACE;}
    HRESULT STDMETHODCALLTYPE GetDisplaySurfaceData1(IDXGIResource* p) noexcept override{return advanced?advanced->GetDisplaySurfaceData1(p):E_NOINTERFACE;}
    HRESULT STDMETHODCALLTYPE DuplicateOutput(IUnknown* device,IDXGIOutputDuplication** out) noexcept override {
        caller(state);++state.v[MFS_LOOKUP_DUPLICATE];
        state.v[MFS_LOOKUP_VIOLATION]|=!out || (out && *out) || reinterpret_cast<uintptr_t>(device)!=state.v[MFS_LOOKUP_DEVICE_POINTER];
        if(!out)return E_POINTER;*out=nullptr;
        state.v[MFS_LOOKUP_SENTINEL_HRESULT]=static_cast<uint32_t>(E_ABORT);return E_ABORT;
    }
};
struct Adapter final : Owner<IDXGIAdapter,Adapter,1> {
    ComPtr<IDXGIAdapter> raw;
    Adapter(State& s,IDXGIAdapter* value) noexcept:Owner(s),raw(value) {s.v[MFS_LOOKUP_ADAPTER_POINTER]=reinterpret_cast<uintptr_t>(static_cast<IDXGIAdapter*>(this));}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) noexcept override {
        caller(state);if(!out)return E_POINTER;*out=nullptr;
        if(iid!=__uuidof(IUnknown) && iid!=__uuidof(IDXGIObject) && iid!=__uuidof(IDXGIAdapter))return E_NOINTERFACE;
        *out=static_cast<IDXGIAdapter*>(this);AddRef();return S_OK;
    }
    OBJECT_FORWARD
    HRESULT STDMETHODCALLTYPE EnumOutputs(UINT index,IDXGIOutput** out) noexcept override {
        caller(state);++state.v[MFS_LOOKUP_ENUM_OUTPUTS];state.v[MFS_LOOKUP_VIOLATION]|=index>1 || !out || (out && *out);
        if(!out)return E_POINTER;*out=nullptr;
        if(index){++state.v[MFS_LOOKUP_OUTPUT_ENDS];state.v[MFS_LOOKUP_VIOLATION]|=fallback(state) || state.v[MFS_LOOKUP_CONTROL] || (state.v[MFS_LOOKUP_MODE]!=2 && state.v[MFS_LOOKUP_MODE]!=4);return DXGI_ERROR_NOT_FOUND;}
        if(fails(state,1))return rejected(state,1);
        ++state.v[MFS_LOOKUP_NATIVE_OUTPUTS];ComPtr<IDXGIOutput> native;auto code=raw->EnumOutputs(index,&native);
        if(FAILED(code) || !native)return FAILED(code)?code:E_UNEXPECTED;
        auto* wrapped=new(std::nothrow) Output(state,native.Get());if(!wrapped)return E_OUTOFMEMORY;
        *out=static_cast<IDXGIOutput*>(wrapped);return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetDesc(DXGI_ADAPTER_DESC* p) noexcept override{return raw->GetDesc(p);}
    HRESULT STDMETHODCALLTYPE CheckInterfaceSupport(REFGUID g,LARGE_INTEGER* p) noexcept override{return raw->CheckInterfaceSupport(g,p);}
};
struct Factory final : Owner<IDXGIFactory,Factory,0> {
    ComPtr<IDXGIFactory> raw;
    Factory(State& s,IDXGIFactory* value) noexcept:Owner(s),raw(value) {s.v[MFS_LOOKUP_FACTORY_POINTER]=reinterpret_cast<uintptr_t>(static_cast<IDXGIFactory*>(this));}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) noexcept override {
        caller(state);if(!out)return E_POINTER;*out=nullptr;
        if(iid!=__uuidof(IUnknown) && iid!=__uuidof(IDXGIObject) && iid!=__uuidof(IDXGIFactory))return E_NOINTERFACE;
        *out=static_cast<IDXGIFactory*>(this);AddRef();return S_OK;
    }
    OBJECT_FORWARD
    HRESULT STDMETHODCALLTYPE EnumAdapters(UINT index,IDXGIAdapter** out) noexcept override {
        caller(state);++state.v[MFS_LOOKUP_ENUM_ADAPTERS];state.v[MFS_LOOKUP_VIOLATION]|=index>1 || !out || (out && *out);
        if(!out)return E_POINTER;*out=nullptr;
        if(index){++state.v[MFS_LOOKUP_ADAPTER_ENDS];state.v[MFS_LOOKUP_VIOLATION]|=fallback(state) || state.v[MFS_LOOKUP_CONTROL] || (state.v[MFS_LOOKUP_MODE]!=1 && state.v[MFS_LOOKUP_MODE]!=2 && state.v[MFS_LOOKUP_MODE]!=4);return DXGI_ERROR_NOT_FOUND;}
        if(++state.zero_adapters==2){
            ++state.v[MFS_LOOKUP_FALLBACKS];
            std::copy_n(state.v.begin()+MFS_LOOKUP_ADAPTER_CREATED,4,state.v.begin()+MFS_LOOKUP_BEFORE_ADAPTER_CREATED);
            state.v[MFS_LOOKUP_VIOLATION]|=state.v[MFS_LOOKUP_ADAPTER_CREATED]!=state.v[MFS_LOOKUP_ADAPTER_DESTROYED] || state.v[MFS_LOOKUP_OUTPUT_CREATED]!=state.v[MFS_LOOKUP_OUTPUT_DESTROYED];
        }
        state.v[MFS_LOOKUP_VIOLATION]|=state.zero_adapters>2;
        if(fails(state,0))return rejected(state,0);
        ++state.v[MFS_LOOKUP_NATIVE_ADAPTERS];ComPtr<IDXGIAdapter> native;auto code=raw->EnumAdapters(index,&native);
        if(FAILED(code) || !native)return FAILED(code)?code:E_UNEXPECTED;
        auto* wrapped=new(std::nothrow) Adapter(state,native.Get());if(!wrapped)return E_OUTOFMEMORY;
        *out=wrapped;return S_OK;
    }
    HRESULT STDMETHODCALLTYPE MakeWindowAssociation(HWND h,UINT f) noexcept override{return raw->MakeWindowAssociation(h,f);}
    HRESULT STDMETHODCALLTYPE GetWindowAssociation(HWND* p) noexcept override{return raw->GetWindowAssociation(p);}
    HRESULT STDMETHODCALLTYPE CreateSwapChain(IUnknown* d,DXGI_SWAP_CHAIN_DESC* s,IDXGISwapChain** p) noexcept override{return raw->CreateSwapChain(d,s,p);}
    HRESULT STDMETHODCALLTYPE CreateSoftwareAdapter(HMODULE m,IDXGIAdapter** p) noexcept override{return raw->CreateSoftwareAdapter(m,p);}
};
#undef OBJECT_FORWARD
}
namespace {
void target_info(HWND window,uint64_t* output) noexcept {
    const bool valid=IsWindow(window)!=FALSE;
    const std::array<uint64_t,7> values{reinterpret_cast<uintptr_t>(window),reinterpret_cast<uintptr_t>(selected_monitor),GetCurrentThreadId(),
        valid?GetWindowThreadProcessId(window,nullptr):0,valid?uint64_t(IsWindowVisible(window)!=FALSE):0,uint64_t(valid),
        valid?reinterpret_cast<uintptr_t>(MonitorFromWindow(window,MONITOR_DEFAULTTONEAREST)):0};
    std::copy(values.begin(),values.end(),output);
}
}
int32_t mfs_dd_lookup_target_create(uint64_t* output,size_t count) noexcept {
    if(!output || count!=7 || active || target)return E_INVALIDARG;
    std::fill_n(output,count,0);ComPtr<IDXGIFactory> factory;ComPtr<IDXGIAdapter> adapter;ComPtr<IDXGIOutput> native_output;
    auto code=CreateDXGIFactory(IID_PPV_ARGS(&factory));if(FAILED(code))return code;
    code=factory->EnumAdapters(0,&adapter);if(FAILED(code))return code;
    code=adapter->EnumOutputs(0,&native_output);if(FAILED(code))return code;
    DXGI_OUTPUT_DESC description{};code=native_output->GetDesc(&description);if(FAILED(code))return code;
    const auto& r=description.DesktopCoordinates;
    if(!description.Monitor || !description.AttachedToDesktop || int64_t(r.right)-r.left<8 || int64_t(r.bottom)-r.top<8)return E_FAIL;
    const auto module=GetModuleHandleW(nullptr);WNDCLASSEXW klass{};klass.cbSize=sizeof(klass);klass.hInstance=module;klass.lpfnWndProc=DefWindowProcW;klass.lpszClassName=target_class;
    if(!RegisterClassExW(&klass))return E_FAIL;
    target=CreateWindowExW(0,target_class,L"hidden lookup control",WS_POPUP,r.left,r.top,8,8,nullptr,nullptr,module,nullptr);
    if(!target){UnregisterClassW(target_class,module);return E_FAIL;}
    selected_monitor=description.Monitor;target_caller=GetCurrentThreadId();target_info(target,output);
    if(output[4] || !output[5] || output[1]!=output[6]){DestroyWindow(target);target=nullptr;UnregisterClassW(target_class,module);return E_FAIL;}
    return S_OK;
}
int32_t mfs_dd_lookup_target_close(uint64_t* output,size_t count) noexcept {
    if(!output || count!=7 || active || !target || GetCurrentThreadId()!=target_caller)return E_INVALIDARG;
    const auto window=target;if(!DestroyWindow(window))return E_FAIL;target=nullptr;
    target_info(window,output);if(!UnregisterClassW(target_class,GetModuleHandleW(nullptr)))return E_FAIL;
    return output[5]?E_FAIL:S_OK;
}
int32_t mfs_dd_lookup_begin(uint32_t mode,uint32_t control,uint32_t index) noexcept {
    if(active || mode>4 || control>1 || index>=states.size() || states[index].used || !IsWindow(target) || GetCurrentThreadId()!=target_caller || MonitorFromWindow(target,MONITOR_DEFAULTTONEAREST)!=selected_monitor)return E_INVALIDARG;
    auto& s=states[index];s.used=true;s.index=index;s.v[MFS_LOOKUP_MODE]=mode;s.v[MFS_LOOKUP_CONTROL]=control;s.v[MFS_LOOKUP_CALLER]=GetCurrentThreadId();s.v[MFS_LOOKUP_TARGET_MONITOR]=reinterpret_cast<uintptr_t>(selected_monitor);active=&s;return S_OK;
}
int32_t mfs_dd_lookup_created(void* device,void* context) noexcept {
    if(!active || !device || !context)return E_INVALIDARG;auto& s=*active;caller(s);
    s.v[MFS_LOOKUP_VIOLATION]|=s.v[MFS_LOOKUP_DEVICES]!=0;++s.v[MFS_LOOKUP_DEVICES];
    s.v[MFS_LOOKUP_DEVICE_POINTER]=reinterpret_cast<uintptr_t>(device);s.v[MFS_LOOKUP_CONTEXT_POINTER]=reinterpret_cast<uintptr_t>(context);
    const auto a=mfs_d3d_lifetime_attach(device,0,s.index*2);const auto b=mfs_d3d_lifetime_attach(context,1,s.index*2+1);
    if(a || b){s.v[MFS_LOOKUP_VIOLATION]=1;return a?a:b;}return S_OK;
}
int32_t mfs_dd_lookup_factory(void* factory,void** output) noexcept {
    if(!active || !factory || !output)return E_INVALIDARG;*output=nullptr;auto& s=*active;caller(s);
    s.v[MFS_LOOKUP_VIOLATION]|=s.v[MFS_LOOKUP_FACTORIES]!=0;++s.v[MFS_LOOKUP_FACTORIES];
    auto* wrapped=new(std::nothrow) Factory(s,static_cast<IDXGIFactory*>(factory));if(!wrapped)return E_OUTOFMEMORY;
    *output=static_cast<IDXGIFactory*>(wrapped);return S_OK;
}
int32_t mfs_dd_lookup_trace(uint64_t* output,size_t capacity) noexcept {
    if(!active || !output || capacity!=MFS_LOOKUP_FIELDS)return E_INVALIDARG;caller(*active);
    std::copy(active->v.begin(),active->v.end(),output);return S_OK;
}
int32_t mfs_dd_lookup_end(void) noexcept {
    if(!active)return E_INVALIDARG;auto& s=*active;caller(s);active=nullptr;
    for(unsigned kind=0;kind<3;++kind)if(s.v[MFS_LOOKUP_FACTORY_CREATED+2*kind]!=s.v[MFS_LOOKUP_FACTORY_DESTROYED+2*kind])return E_UNEXPECTED;
    return s.v[MFS_LOOKUP_VIOLATION]?E_UNEXPECTED:S_OK;
}
