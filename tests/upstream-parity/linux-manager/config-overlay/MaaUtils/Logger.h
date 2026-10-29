// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
namespace MfsConfigLog {
struct NullLog {template<class T> const NullLog& operator<<(const T&) const noexcept {return *this;}};
inline const NullLog sink;
}
// Explicit configuration oracle logging suppression; no timing claim.
#define LogFunc MfsConfigLog::sink
#define LogError MfsConfigLog::sink
#define VAR(x) x
#define VAR_VOIDP(x) x
