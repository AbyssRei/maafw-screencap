// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Logging only. Real public interfaces, manager, buffers and API are unchanged.
namespace MfsCustomNativeLog {
struct Sink {template<class T> const Sink& operator<<(const T&) const noexcept{return *this;}};
inline const Sink sink;
}
#define LogFunc MfsCustomNativeLog::sink
#define LogTrace MfsCustomNativeLog::sink
#define LogWarn MfsCustomNativeLog::sink
#define LogError MfsCustomNativeLog::sink
#define VAR(...) 0
#define VAR_VOIDP(...) 0
