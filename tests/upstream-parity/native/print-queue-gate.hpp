// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <array>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <mutex>

// Acceptance intervention only: stop the owned UI queue before it consumes a
// real SWP_ASYNCWINDOWPOS request. No window procedure/native request is replaced.
// The caller releases this gate directly, never through the blocked UI queue.
// A lost caller is bounded and recorded as failure, not accepted as completion.
struct MfsPrintQueueGate {
    std::mutex mutex;
    std::condition_variable changed;
    bool armed = false, entered = false, released = false, exited = false, timed_out = false;

    bool arm()
    {
        std::lock_guard lock(mutex);
        if (armed) { return false; }
        armed = true;
        return true;
    }
    bool await_entry()
    {
        std::unique_lock lock(mutex);
        if (!changed.wait_for(lock, std::chrono::seconds(10), [&] { return entered; })) {
            timed_out = true;
            released = true;
            changed.notify_all();
            return false;
        }
        return !timed_out && !released;
    }
    void hold()
    {
        std::unique_lock lock(mutex);
        entered = true;
        changed.notify_all();
        if (!changed.wait_for(lock, std::chrono::seconds(10), [&] { return released; })) {
            timed_out = true;
            released = true;
        }
        exited = true;
        changed.notify_all();
    }
    bool release()
    {
        std::unique_lock lock(mutex);
        if (!armed) { return false; }
        released = true;
        changed.notify_all();
        if (!changed.wait_for(lock, std::chrono::seconds(10), [&] { return exited; })) {
            timed_out = true;
            return false;
        }
        return !timed_out;
    }
    std::array<uint64_t, 5> state()
    {
        std::lock_guard lock(mutex);
        return { armed, entered, released, exited, timed_out };
    }
};
