// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#define MFS_FRAMEPOOL_INIT_EXPORTS
#include "framepool-init-fixture.h"
#include <Windows.h>
#include <d3d11.h>
#include <array>
#include <atomic>
#include <new>

namespace {
struct Record {
    std::atomic<uint64_t> attached{0}, final{0}, attach_thread{0}, final_thread{0}, attach_qpc{0}, final_qpc{0};
};
// No native callback borrows a stack object or an output stream. The DLL stays
// loaded for the isolated acceptance process, including every failure exit.
std::array<Record,40> records;
uint64_t qpc() noexcept { LARGE_INTEGER value{};QueryPerformanceCounter(&value);return static_cast<uint64_t>(value.QuadPart); }
struct Tracker final : IUnknown {
    std::atomic<ULONG> refs{1};const uint32_t index;
    explicit Tracker(uint32_t value) noexcept : index(value) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** output) noexcept override {
        if(!output){return E_POINTER;}*output=nullptr;
        if(iid!=__uuidof(IUnknown)){return E_NOINTERFACE;}
        *output=static_cast<IUnknown*>(this);AddRef();return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() noexcept override {return ++refs;}
    ULONG STDMETHODCALLTYPE Release() noexcept override {
        const auto remaining=--refs;
        if(!remaining){
            auto& record=records[index];record.final_thread.store(GetCurrentThreadId());record.final_qpc.store(qpc());
            record.final.fetch_add(1,std::memory_order_release);delete this;
        }
        return remaining;
    }
};
constexpr GUID tracker_id{0x688b7497,0x1476,0x48b6,{0x8d,0x51,0x17,0x4d,0xde,0x51,0x4c,0x38}};
}
uint32_t mfs_framepool_init_version() noexcept {return 1;}
uint64_t mfs_framepool_init_clock() noexcept {return qpc();}
int32_t mfs_framepool_init_widths(uint32_t* output,size_t count) noexcept {
    if(!output || count!=6){return E_INVALIDARG;}
    const std::array<uint32_t,6> values{sizeof(void*),sizeof(size_t),sizeof(uint64_t),sizeof(int32_t),sizeof(uint32_t),sizeof(HRESULT)};
    for(size_t i=0;i<count;++i){output[i]=values[i];}return S_OK;
}
int32_t mfs_framepool_init_window_create(void** output) noexcept {
    if(!output){return E_POINTER;}*output=nullptr;
    const auto window=CreateWindowExW(0,L"STATIC",L"mfs hidden initialization",WS_POPUP,
        10,10,96,64,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
    if(!window){return HRESULT_FROM_WIN32(GetLastError());}
    if(IsWindowVisible(window)){DestroyWindow(window);return E_UNEXPECTED;}
    *output=window;return S_OK;
}
int32_t mfs_framepool_init_window_destroy(void** window) noexcept {
    if(!window || !*window){return E_INVALIDARG;}
    const auto value=static_cast<HWND>(*window);
    if(GetWindowThreadProcessId(value,nullptr)!=GetCurrentThreadId()){return E_ACCESSDENIED;}
    if(!DestroyWindow(value)){return HRESULT_FROM_WIN32(GetLastError());}*window=nullptr;return S_OK;
}
int32_t mfs_framepool_init_window_info(void* window,uint64_t* output,size_t count) noexcept {
    if(!window || !output || count!=4){return E_INVALIDARG;}
    const auto value=static_cast<HWND>(window);
    output[0]=GetCurrentThreadId();output[1]=GetWindowThreadProcessId(value,nullptr);
    output[2]=IsWindow(value)!=FALSE;output[3]=IsWindowVisible(value)!=FALSE;return S_OK;
}
int32_t mfs_framepool_init_attach(void* device,uint32_t index) noexcept {
    if(!device || index>=records.size()){return E_INVALIDARG;}
    auto& record=records[index];
    uint64_t expected=0;
    if(!record.attached.compare_exchange_strong(expected,1)){return E_UNEXPECTED;}
    auto* tracker=new(std::nothrow) Tracker(index);
    if(!tracker){return E_OUTOFMEMORY;}
    record.attach_thread.store(GetCurrentThreadId());record.attach_qpc.store(qpc());
    // The caller retains the real live device. A successful native call adds
    // its own sentinel reference; release only this factory's initial +1.
    const auto result=static_cast<ID3D11Device*>(device)->SetPrivateDataInterface(tracker_id,tracker);
    tracker->Release();return result;
}
int32_t mfs_framepool_init_record(uint32_t index,uint64_t* output,size_t count) noexcept {
    if(index>=records.size() || !output || count!=6){return E_INVALIDARG;}
    auto& r=records[index];
    const auto final=r.final.load(std::memory_order_acquire);
    output[0]=r.attached.load();output[1]=final;output[2]=r.attach_thread.load();output[3]=r.final_thread.load();
    output[4]=r.attach_qpc.load();output[5]=r.final_qpc.load();return S_OK;
}
