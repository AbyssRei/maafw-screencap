// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <memory>
#include <utility>

namespace MfsMacSample {
// Each escaping native completion captures a shared owner. The thread-local
// value is only a synchronous association while forwarding that completion;
// it never owns or borrows a caller's stack/context/native sample.
template<class T> class ThreadScope {
public:
    explicit ThreadScope(std::shared_ptr<T> value) : previous_(std::exchange(slot(),std::move(value))) {}
    ~ThreadScope() {slot()=std::move(previous_);}
    ThreadScope(const ThreadScope&)=delete;
    ThreadScope& operator=(const ThreadScope&)=delete;
    static std::shared_ptr<T> current() {return slot();}
private:
    static std::shared_ptr<T>& slot() {static thread_local std::shared_ptr<T> value;return value;}
    std::shared_ptr<T> previous_;
};
}
