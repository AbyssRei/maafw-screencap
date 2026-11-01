// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Borrowed expression views over the original fields, never replacement COM
// objects. Every operation forwards to the same real native interface.
namespace mfs_dup_map {
struct ConversionFailure : std::exception {
    const char* what() const noexcept override { return "controlled desktop conversion exception"; }
};
struct Trace {
    uint32_t fault=0, order=0, acquires=0, copies=0, maps=0, unmaps=0, releases=0, conversions=0, thread=0;
    int32_t acquired=0, mapped_result=0, released=0;
    uint32_t timeout=0, subresource=0, pitch=0;
    bool mapped=false, held=false, violation=false;
    uintptr_t context=0, resource=0, duplication=0;
};
inline thread_local Trace* active=nullptr;
inline Trace* event(uint32_t code) noexcept {
    if(!active)return nullptr;
    auto& t=*active;
    const auto thread=GetCurrentThreadId();
    if(!t.thread)t.thread=thread;else t.violation|=t.thread!=thread;
    if(t.order>(UINT32_MAX-code)/10)t.violation=true;else t.order=t.order*10+code;
    return &t;
}
struct Scope {
    explicit Scope(Trace& t){if(active)throw std::runtime_error("nested desktop observer");active=&t;}
    ~Scope(){active=nullptr;}
    Scope(const Scope&)=delete;Scope& operator=(const Scope&)=delete;
};
struct Context {
    ID3D11DeviceContext*& value;
    explicit operator bool()const noexcept{return value!=nullptr;}
    ID3D11DeviceContext** operator&()noexcept{return &value;}
    Context* operator->()noexcept{return this;}
    Context& operator=(std::nullptr_t)noexcept{value=nullptr;return *this;}
    ULONG Release()noexcept{return value->Release();}
    void CopyResource(ID3D11Resource* target,ID3D11Resource* source) {
        if(auto* t=event(2)){
            ++t->copies;t->context=reinterpret_cast<uintptr_t>(value);t->resource=reinterpret_cast<uintptr_t>(target);
            t->violation|=!t->held || t->mapped;
        }
        value->CopyResource(target,source);
    }
    HRESULT Map(ID3D11Resource* target,UINT index,D3D11_MAP kind,UINT flags,D3D11_MAPPED_SUBRESOURCE* output) {
        auto* t=event(3);
        const auto actual=t && t->fault==1 ? UINT32_MAX:index;
        if(t){++t->maps;t->subresource=actual;t->violation|=t->mapped || !t->held || t->context!=reinterpret_cast<uintptr_t>(value) || t->resource!=reinterpret_cast<uintptr_t>(target);}
        const HRESULT code=value->Map(target,actual,kind,flags,output);
        if(actual!=index && SUCCEEDED(code)){
            value->Unmap(target,actual);
            throw std::runtime_error("invalid-subresource desktop Map unexpectedly succeeded");
        }
        if(t){t->mapped_result=code;t->mapped=SUCCEEDED(code);t->pitch=SUCCEEDED(code)?output->RowPitch:0;}
        return code;
    }
    void Unmap(ID3D11Resource* target,UINT index)noexcept {
        value->Unmap(target,index);
        if(auto* t=event(4)){
            ++t->unmaps;t->violation|=!t->mapped || !t->held || index || t->context!=reinterpret_cast<uintptr_t>(value) || t->resource!=reinterpret_cast<uintptr_t>(target);t->mapped=false;
        }
    }
};
inline Context context(ID3D11DeviceContext*& value)noexcept{return {value};}
struct Duplication {
    IDXGIOutputDuplication*& value;
    explicit operator bool()const noexcept{return value!=nullptr;}
    IDXGIOutputDuplication** operator&()noexcept{return &value;}
    Duplication* operator->()noexcept{return this;}
    Duplication& operator=(std::nullptr_t)noexcept{value=nullptr;return *this;}
    ULONG Release()noexcept{return value->Release();}
    HRESULT AcquireNextFrame(UINT timeout,DXGI_OUTDUPL_FRAME_INFO* info,IDXGIResource** output) {
        auto* t=event(1);
        if(t){++t->acquires;t->timeout=timeout;t->duplication=reinterpret_cast<uintptr_t>(value);t->violation|=t->held || t->mapped;}
        const HRESULT code=value->AcquireNextFrame(timeout,info,output);
        if(t){t->acquired=code;t->held=SUCCEEDED(code);}
        return code;
    }
    HRESULT ReleaseFrame()noexcept {
        const HRESULT code=value->ReleaseFrame();
        if(auto* t=event(5)){++t->releases;t->released=code;t->violation|=!t->held || t->mapped || t->duplication!=reinterpret_cast<uintptr_t>(value);t->held=false;}
        return code;
    }
};
inline Duplication duplication(IDXGIOutputDuplication*& value)noexcept{return {value};}
template<class T>inline T* pointer(T* value)noexcept{return value;}
inline ID3D11DeviceContext* pointer(Context value)noexcept{return value.value;}
inline IDXGIOutputDuplication* pointer(Duplication value)noexcept{return value.value;}
inline cv::Mat convert(const cv::Mat& input) {
    if(auto* t=event(6)){
        ++t->conversions;t->violation|=t->held || t->mapped;
        if(t->fault==2)throw ConversionFailure{};
    }
    return MAA_CTRL_UNIT_NS::bgra_to_bgr(input);
}
}
