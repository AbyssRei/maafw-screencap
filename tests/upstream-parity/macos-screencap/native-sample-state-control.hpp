// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "native-sample-state.hpp"
#include <atomic>
#include <cstdint>
namespace MfsSampleControl {
struct State {
    explicit State(uint64_t value):id(value) {++live;}
    ~State() {--live;}
    const uint64_t id;
    inline static std::atomic<unsigned> live{0};
};
using Scope=MfsMacSample::ThreadScope<State>;
uint64_t other_translation_unit_id();
}
