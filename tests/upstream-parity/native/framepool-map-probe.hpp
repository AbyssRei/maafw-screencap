// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Nonowning expression views forward the original class's real interfaces and
// Mat field. They are not COM implementations and never replace a native vtable.
namespace mfs_map_probe {
struct ConversionFailure : std::exception {
    const char* what() const noexcept override { return "controlled conversion exception"; }
};
struct Trace {
    bool active = false, mapped = false, violation = false;
    uint32_t fault = 0, order = 0, maps = 0, unmaps = 0, conversions = 0, cache_clones = 0;
    int32_t map_hresult = 0;
    uint32_t subresource = 0, pitch = 0, thread = 0;
    uintptr_t context = 0, resource = 0;
};
struct Owner { Trace trace; cv::Mat* cache = nullptr; };
inline thread_local Owner* owner = nullptr;
inline void increment(uint32_t& value, Trace& trace) noexcept {
    if (value == UINT32_MAX) { trace.violation = true; } else { ++value; }
}
struct Scope {
    explicit Scope(Owner& value) { if (owner) { throw std::runtime_error("nested map observer"); } owner = &value; }
    ~Scope() noexcept { owner = nullptr; }
    Scope(const Scope&) = delete;
    Scope& operator=(const Scope&) = delete;
};
inline Trace* event(uint32_t code) noexcept {
    if (!owner || !owner->trace.active) { return nullptr; }
    auto& trace = owner->trace;
    const auto thread = GetCurrentThreadId();
    if (!trace.thread) { trace.thread = thread; }
    else if (trace.thread != thread) { trace.violation = true; }
    if (trace.order > (UINT32_MAX-code)/10) { trace.violation = true; }
    else { trace.order = trace.order*10+code; }
    return &trace;
}
struct ContextView {
    winrt::com_ptr<ID3D11DeviceContext>& value;
    auto put() { return value.put(); }
    ContextView* operator->() noexcept { return this; }
    void CopyResource(ID3D11Resource* target, ID3D11Resource* source) {
        if (auto* trace = event(1)) {
            trace->context = reinterpret_cast<uintptr_t>(value.get());
            trace->resource = reinterpret_cast<uintptr_t>(target);
        }
        value->CopyResource(target, source);
    }
    HRESULT Map(ID3D11Resource* target, UINT index, D3D11_MAP kind, UINT flags, D3D11_MAPPED_SUBRESOURCE* output) {
        auto* trace = event(2);
        const auto actual = trace && trace->fault == 1 ? UINT32_MAX : index;
        if (trace) {
            increment(trace->maps, *trace); trace->subresource = actual;
            trace->violation |= trace->mapped || trace->context != reinterpret_cast<uintptr_t>(value.get())
                || trace->resource != reinterpret_cast<uintptr_t>(target);
        }
        const HRESULT result = value->Map(target, actual, kind, flags, output);
        if (actual != index && SUCCEEDED(result)) {
            value->Unmap(target, actual);
            throw std::runtime_error("invalid-subresource Map unexpectedly succeeded");
        }
        if (trace) {
            trace->map_hresult = result; trace->mapped = SUCCEEDED(result);
            // Failed output is deliberately not consumed.
            trace->pitch = SUCCEEDED(result) ? output->RowPitch : 0;
        }
        return result;
    }
    void Unmap(ID3D11Resource* target, UINT index) noexcept {
        value->Unmap(target, index);
        if (auto* trace = event(5)) {
            increment(trace->unmaps, *trace);
            trace->violation |= !trace->mapped || index || trace->context != reinterpret_cast<uintptr_t>(value.get())
                || trace->resource != reinterpret_cast<uintptr_t>(target);
            trace->mapped = false;
        }
    }
};
inline ContextView context(winrt::com_ptr<ID3D11DeviceContext>& value) { return { value }; }
struct CacheView {
    cv::Mat& value;
    explicit CacheView(cv::Mat& field) : value(field) {
        if (owner) {
            if (owner->cache && owner->cache != &field) { owner->trace.violation = true; }
            owner->cache = &field;
        }
    }
    bool empty() const { return value.empty(); }
    cv::Mat clone() const { return value.clone(); }
    CacheView& operator=(cv::Mat&& image) {
        value = std::move(image);
        if (auto* trace = event(4)) { increment(trace->cache_clones, *trace); trace->violation |= !trace->mapped; }
        return *this;
    }
};
inline CacheView cache(cv::Mat& value) { return CacheView(value); }
inline cv::Mat convert(const cv::Mat& raw) {
    if (auto* trace = event(3)) {
        increment(trace->conversions, *trace); trace->violation |= !trace->mapped;
        if (trace->fault == 2) { throw ConversionFailure {}; }
    }
    return MAA_CTRL_UNIT_NS::bgra_to_bgr(raw);
}
}
