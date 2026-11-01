// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Test-only observation. Each wrapper forwards exactly one original operation;
// no frame is retained here, no native query is added and no wait is changed.
#pragma once
#include <cassert>
#include <cstdint>
#include <limits>
#include <winrt/Windows.Graphics.Capture.h>
#include "MaaUtils/NoWarningCV.hpp"

namespace mfs_poll_probe {
struct Counts {
    uint64_t drain_calls = 0, drain_frames = 0, poll_calls = 0, poll_frames = 0;
    uint64_t cache_clones = 0, errors = 0;
};
inline thread_local Counts* current = nullptr;
inline void increment(uint64_t& value) noexcept
{
    // Saturation becomes invalid records, never an exception that changes the
    // original native operation or its existing exception boundary.
    if (value != (std::numeric_limits<uint64_t>::max)()) { ++value; }
}
struct Scope {
    Counts counts;
    Scope() { assert(current == nullptr); current = &counts; }
    ~Scope() { current = nullptr; }
    Scope(const Scope&) = delete;
    Scope& operator=(const Scope&) = delete;
};
inline auto next(const winrt::Windows::Graphics::Capture::Direct3D11CaptureFramePool& pool, bool drain)
{
    assert(current);
    increment(drain ? current->drain_calls : current->poll_calls);
    try {
        auto frame = pool.TryGetNextFrame();
        if (frame) { increment(drain ? current->drain_frames : current->poll_frames); }
        return frame;
    }
    catch (...) { increment(current->errors); throw; }
}
inline cv::Mat clone(const cv::Mat& cached)
{
    assert(current);
    try {
        auto image = cached.clone();
        increment(current->cache_clones);
        return image;
    }
    catch (...) { increment(current->errors); throw; }
}
} // namespace mfs_poll_probe
