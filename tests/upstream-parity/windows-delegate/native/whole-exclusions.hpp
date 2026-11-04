// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <atomic>
#include <cstdint>
#include <stdexcept>
// Explicit acceptance boundaries for excluded non-None input construction and
// application-string conversion. Capture and the original empty background-key
// owner use their real F classes. No successful stand-in input can be created.
namespace MfsWhole {
inline std::atomic<uint64_t> input_calls{0},application_calls{0};
[[noreturn]] inline void input() {++input_calls;throw std::logic_error("excluded non-None input");}
[[noreturn]] inline void application() {++application_calls;throw std::logic_error("excluded application conversion");}
}
