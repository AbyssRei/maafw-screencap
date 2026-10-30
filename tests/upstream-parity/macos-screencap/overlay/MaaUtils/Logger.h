// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Reference logging boundary only; every original native call is retained.
namespace MfsMacLog {
struct NullLog {
    template<class T> constexpr const NullLog& operator<<(const T&) const noexcept { return *this; }
};
inline constexpr NullLog sink;
}
#ifdef MFS_MAC_DISPATCH_OBSERVER
#define LogError MfsMacObserve::errors
#else
#define LogError MfsMacLog::sink
#endif
