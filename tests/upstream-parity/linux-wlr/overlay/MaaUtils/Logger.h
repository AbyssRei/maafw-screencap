// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Explicit reference-only logging boundary, not a latency benchmark.
namespace MfsWlrLog {
struct NullLog {
    template<class T> constexpr const NullLog& operator<<(const T&) const noexcept { return *this; }
};
inline constexpr NullLog sink;
}
#define LogTrace MfsWlrLog::sink
#define LogDebug MfsWlrLog::sink
#define LogInfo MfsWlrLog::sink
#define LogWarn MfsWlrLog::sink
#define LogError MfsWlrLog::sink
#define VAR(x) x
#define VAR_VOIDP(x) static_cast<const void*>(x)
