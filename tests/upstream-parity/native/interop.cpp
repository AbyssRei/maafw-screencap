// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "interop.h"
#include <algorithm>
#include <array>
#include <new>
#include <thread>
#include <opencv2/core.hpp>
#include <opencv2/imgproc.hpp>

namespace {
struct Worker {
    std::thread thread;
};
thread_local Worker* active_callback = nullptr;
}

int32_t mfs_p0_start(void* context, MfsP0Callback callback, uint32_t fault, void** output)
{
    if (!output) {
        return 1;
    }
    *output = nullptr;
    if (!context || !callback || fault > 2) {
        return 1;
    }
    Worker* worker = nullptr;
    try {
        if (fault == 1) { throw std::bad_alloc(); }
        worker = new (std::nothrow) Worker;
        if (!worker) { return 3; }
        worker->thread = std::thread([worker, context, callback, fault] {
            // Allocate and convert before callback publication. Rust must copy
            // while these native pixels are alive and must not retain the span.
            try {
                if (fault == 2) {
                    throw cv::Exception(cv::Error::StsBadArg, "injected conversion failure", "P0", __FILE__, __LINE__);
                }
                std::array<uint8_t, 16> pixels { 1, 2, 3, 255, 4, 5, 6, 0, 7, 8, 9, 128, 10, 11, 12, 64 };
                cv::Mat bgra(2, 2, CV_8UC4, pixels.data(), 8);
                cv::Mat bgr;
                cv::cvtColor(bgra, bgr, cv::COLOR_BGRA2BGR);
                MfsP0Span span { bgr.data, bgr.total() * bgr.elemSize(), 2, 2, static_cast<uint32_t>(bgr.step), 3 };
                active_callback = worker;
                callback(context, worker, &span, 0);
                active_callback = nullptr;
                std::fill_n(bgr.data, span.length, uint8_t { 0 });
            }
            catch (...) {
                active_callback = worker;
                callback(context, worker, nullptr, 3);
                active_callback = nullptr;
            }
        });
    }
    catch (...) {
        delete worker;
        return 3;
    }
    *output = worker;
    return 0;
}

int32_t mfs_p0_stop(void* handle)
{
    if (!handle) {
        return 1;
    }
    auto* worker = static_cast<Worker*>(handle);
    if (active_callback == worker) {
        return 2;
    }
    try {
        worker->thread.join();
    }
    catch (...) {
        // The caller still owns the handle/context/module on cleanup failure.
        return 3;
    }
    delete worker;
    return 0;
}

int32_t mfs_p0_layout(uint64_t* output, size_t capacity)
{
    constexpr std::array<uint64_t, 8> layout {
        sizeof(MfsP0Span), alignof(MfsP0Span), offsetof(MfsP0Span, data), offsetof(MfsP0Span, length),
        offsetof(MfsP0Span, width), offsetof(MfsP0Span, height), offsetof(MfsP0Span, stride), offsetof(MfsP0Span, channels)
    };
    if (!output || capacity < layout.size()) {
        return 1;
    }
    std::copy(layout.begin(), layout.end(), output);
    return 0;
}

const char* mfs_p0_opencv_info(void)
{
    try {
        return cv::getBuildInformation().c_str();
    }
    catch (...) {
        return nullptr;
    }
}
