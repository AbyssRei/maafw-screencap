// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#define MFS_D3D_LIFETIME_EXPORTS
#include "d3d-lifetime.h"
#include <Windows.h>
#include <d3d11.h>
#include <dxgi1_2.h>
#include <array>
#include <atomic>
#include <new>
namespace {
struct Record {std::array<std::atomic<uint64_t>,12> value{};};
std::array<Record,80> records;
uint64_t clock_value() noexcept {LARGE_INTEGER v{};QueryPerformanceCounter(&v);return static_cast<uint64_t>(v.QuadPart);}
void CALLBACK finished(void* context) noexcept {
    auto& r=*static_cast<Record*>(context);
    r.value[6].store(GetCurrentThreadId());r.value[8].store(clock_value());
    r.value[4].fetch_add(1,std::memory_order_release);
}
struct PrivateOwner final : IUnknown {
    std::atomic<ULONG> refs{1};Record& record;
    explicit PrivateOwner(Record& r) noexcept:record(r){}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) noexcept override {
        if(!out){return E_POINTER;}*out=nullptr;if(iid!=__uuidof(IUnknown)){return E_NOINTERFACE;}
        *out=static_cast<IUnknown*>(this);AddRef();return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() noexcept override {return ++refs;}
    ULONG STDMETHODCALLTYPE Release() noexcept override {
        const auto count=--refs;if(!count){finished(&record);delete this;}return count;
    }
};
constexpr GUID owner_guid{0xc4469e36,0xecaa,0x41ef,{0xb2,0x63,0x1c,0xcf,0x18,0x59,0x39,0x4b}};
}
uint32_t mfs_d3d_lifetime_version() noexcept {return 1;}
uint64_t mfs_d3d_lifetime_clock() noexcept {return clock_value();}
int32_t mfs_d3d_lifetime_widths(uint32_t* out,size_t count) noexcept {
    if(!out || count!=6){return E_INVALIDARG;}
    const std::array<uint32_t,6> values{sizeof(void*),sizeof(size_t),sizeof(uint64_t),sizeof(uint32_t),sizeof(HRESULT),sizeof(UINT)};
    for(size_t i=0;i<count;++i){out[i]=values[i];}return S_OK;
}
int32_t mfs_d3d_lifetime_attach(void* object,uint32_t kind,uint32_t index) noexcept {
    if(!object || kind>3 || index>=records.size()){return E_INVALIDARG;}
    auto& r=records[index];uint64_t empty=0;
    if(!r.value[0].compare_exchange_strong(empty,1)){return E_UNEXPECTED;}
    r.value[1].store(kind==2?2:1);r.value[2].store(kind);r.value[5].store(GetCurrentThreadId());
    r.value[7].store(clock_value());r.value[10].store(reinterpret_cast<uintptr_t>(object));
    const std::array<IID,4> types{__uuidof(ID3D11Device),__uuidof(ID3D11DeviceContext),__uuidof(IDXGIOutputDuplication),__uuidof(ID3D11Texture2D)};
    IUnknown* typed=nullptr;
    const auto checked=static_cast<IUnknown*>(object)->QueryInterface(types[kind],reinterpret_cast<void**>(&typed));
    if(FAILED(checked) || !typed){r.value[11].store(static_cast<uint32_t>(FAILED(checked)?checked:E_UNEXPECTED));return FAILED(checked)?checked:E_UNEXPECTED;}
    struct ReleaseType {IUnknown* value;~ReleaseType(){value->Release();}} release_type{typed};
    HRESULT status=E_UNEXPECTED;
    if(kind==2){
        auto* owner=new(std::nothrow) PrivateOwner(r);
        if(!owner){status=E_OUTOFMEMORY;}
        else {
            // Actual IDXGIOutputDuplication inherits IDXGIObject. This GUID is
            // never overwritten; final owner release is recorded separately
            // from the D3D notifier's stronger destruction-order guarantee.
            status=static_cast<IDXGIOutputDuplication*>(typed)->SetPrivateDataInterface(owner_guid,owner);
            r.value[3].store(SUCCEEDED(status));owner->Release();
        }
    }else{
        ID3DDestructionNotifier* notifier=nullptr;
        // Each supplied concrete COM interface has an IUnknown prefix. QI adds
        // a temporary reference; release it before returning, otherwise this
        // observation would itself prevent destruction of the observed object.
        status=typed->QueryInterface(__uuidof(ID3DDestructionNotifier),reinterpret_cast<void**>(&notifier));
        if(SUCCEEDED(status) && notifier){
            UINT id=0;status=notifier->RegisterDestructionCallback(finished,&r,&id);
            r.value[9].store(id);r.value[3].store(SUCCEEDED(status));notifier->Release();
        }else if(SUCCEEDED(status)){status=E_UNEXPECTED;}
    }
    r.value[11].store(static_cast<uint32_t>(status));return status;
}
int32_t mfs_d3d_lifetime_read(uint32_t index,uint64_t* out,size_t count) noexcept {
    if(index>=records.size() || !out || count!=12){return E_INVALIDARG;}
    auto& r=records[index];const auto done=r.value[4].load(std::memory_order_acquire);
    for(size_t i=0;i<count;++i){out[i]=i==4?done:r.value[i].load();}return S_OK;
}
