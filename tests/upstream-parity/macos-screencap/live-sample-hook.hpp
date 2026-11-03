// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <cstdint>
#include <utility>
namespace MfsMacSample {
// Kept platform-independent so the exact transfer/destruction guard can also
// be exercised on the development host. No borrowed caller stack is stored.
struct LiveHook {
    using Visit=int32_t (*)(void*,const void*,uint32_t) noexcept;
    using Release=void (*)(void*) noexcept;
    void* context;
    Visit visit;
    Release release;
    LiveHook(void* c,Visit v,Release r) noexcept : context(c),visit(v),release(r) {}
    LiveHook(LiveHook&& old) noexcept
        : context(old.context),visit(old.visit),release(std::exchange(old.release,nullptr)) {}
    LiveHook(const LiveHook&)=delete;
    LiveHook& operator=(const LiveHook&)=delete;
    ~LiveHook() {if(release) {release(context);}}
};
}
