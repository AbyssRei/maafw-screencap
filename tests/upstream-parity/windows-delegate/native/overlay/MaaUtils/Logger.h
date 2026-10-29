// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "MaaUtils/ScopeLeave.hpp"
// Logger only; native loading, resource APIs and capture are never overlaid.
namespace MfsNativeLog {
struct Sink { template<class T> const Sink& operator<<(const T&) const noexcept { return *this; } };
inline const Sink sink;
}
#define LogFunc MfsNativeLog::sink
#define LogError MfsNativeLog::sink
#define LogInfo MfsNativeLog::sink
#define LogDebug MfsNativeLog::sink
#define LogWarn MfsNativeLog::sink
#define VAR(...) 0
#define VAR_VOIDP(...) 0
