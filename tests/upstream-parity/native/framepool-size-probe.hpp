// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
namespace mfs_size_probe {
constexpr int32_t injected = static_cast<int32_t>(0x80004004u);
struct Query {
    int32_t native_code = 0, width = 0, height = 0, returned_code = 0, returned_width = 0, returned_height = 0;
    uint32_t thread = 0;
    uintptr_t item = 0;
};
struct Trace { uint32_t mode = 0, calls = 0; std::array<Query, 8> queries {}; };
inline thread_local Trace* active = nullptr;
struct Scope {
    explicit Scope(Trace& trace) { if (active) { throw std::runtime_error("nested Size observer"); } active = &trace; }
    ~Scope() noexcept { active = nullptr; }
    Scope(const Scope&) = delete;
    Scope& operator=(const Scope&) = delete;
};
inline winrt::Windows::Graphics::SizeInt32 size(const winrt::Windows::Graphics::Capture::GraphicsCaptureItem& item)
{
    // A real item query completes before any intervention. A later synthetic
    // throw/result verifies control flow; it is not an observed native failure.
    if (!active) { return item.Size(); }
    auto& trace = *active;
    if (trace.calls >= trace.queries.size()) { throw std::runtime_error("extra Size query"); }
    auto& query = trace.queries[trace.calls++];
    query.thread = GetCurrentThreadId(); query.item = reinterpret_cast<uintptr_t>(winrt::get_abi(item));
    winrt::Windows::Graphics::SizeInt32 value {};
    try { value = item.Size(); }
    catch (const winrt::hresult_error& error) { query.native_code = query.returned_code = error.code(); throw; }
    query.width = value.Width; query.height = value.Height;
    const uint32_t fault = trace.mode >= 1 && trace.mode <= 3 ? trace.mode
        : trace.mode == 4 ? 1 : trace.mode == 5 ? 3 : trace.mode == 6 ? 4 : trace.mode == 7 ? 2 : 0;
    if (trace.calls == fault) { query.returned_code = injected; winrt::throw_hresult(injected); }
    if ((trace.mode == 8 && trace.calls == 1) || (trace.mode == 9 && trace.calls == 2)) { value.Width = 0; }
    else if (trace.mode >= 5 && trace.mode <= 7 && trace.calls == 1) { ++value.Width; }
    query.returned_width = value.Width; query.returned_height = value.Height;
    return value;
}
inline uint32_t slots(const MfsFrameQueryOriginal::FramePoolScreencap& value)
{
    return (value.d3d_device_ ? 1 : 0) | (value.d3d_context_ ? 2 : 0) | (value.dxgi_swap_chain_ ? 4 : 0)
        | (value.readable_texture_ ? 8 : 0) | (value.cap_item_ ? 16 : 0) | (value.cap_frame_pool_ ? 32 : 0)
        | (value.cap_session_ ? 64 : 0);
}
}
