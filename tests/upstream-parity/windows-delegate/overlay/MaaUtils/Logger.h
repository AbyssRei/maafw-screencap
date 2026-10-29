// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
namespace MfsDelegateLog {
struct Null { template<class T> const Null& operator<<(const T&) const noexcept { return *this; } };
inline const Null sink;
}
#define LogFunc MfsDelegateLog::sink
#define LogError MfsDelegateLog::sink
#define LogInfo MfsDelegateLog::sink
#define LogDebug MfsDelegateLog::sink
#define LogWarn MfsDelegateLog::sink
#define VAR(...) 0
#define VAR_VOIDP(...) 0
