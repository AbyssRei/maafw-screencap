// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Explicit test-only logging suppression, not a native timing comparison.
namespace MfsPipeWireLog {
struct NullLog {
    template<class T> constexpr const NullLog& operator<<(const T&) const noexcept { return *this; }
};
inline constexpr NullLog sink;
}
#define LogTrace MfsPipeWireLog::sink
#define LogDebug MfsPipeWireLog::sink
#define LogInfo MfsPipeWireLog::sink
#define LogWarn MfsPipeWireLog::sink
#define LogError MfsPipeWireLog::sink
#define VAR(x) x
