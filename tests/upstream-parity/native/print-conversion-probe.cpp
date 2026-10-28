// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Only the original cvtColor call is interposed. Native capture/read/cleanup
// still execute in the exact fixed source, in an isolated namespace/TU.
#include "print-parity.h"
#include "gdi-probe-internal.hpp"
#include "Base/UnitBase.h"
#include "Common/Conf.h"
#include "MaaUtils/Logger.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <new>
#include <mutex>

namespace {
struct ConversionProbe {
    bool active = false;
    uint32_t fail_at = 0;
    std::array<uint64_t, 12> report {};
};
thread_local ConversionProbe conversion;
// Observe only the intermediate BGR output, without replacing the process-wide
// allocator. Fault modes never export a Mat with this module-owned allocator.
class BgrAllocator final : public cv::MatAllocator {
public:
    mutable std::atomic<cv::UMatData*> live { nullptr };
    mutable std::atomic<uint64_t> allocations { 0 }, releases { 0 }, bytes { 0 }, freed_bytes { 0 }, errors { 0 };
    bool reset()
    {
        if (live.load() || cv::Mat::getDefaultAllocator() != cv::Mat::getStdAllocator()) { return false; }
        allocations = 0; releases = 0; bytes = 0; freed_bytes = 0; errors = 0;
        return true;
    }
    cv::UMatData* allocate(int dims, const int* sizes, int type, void* data, size_t* step,
        cv::AccessFlag flags, cv::UMatUsageFlags usage) const override
    {
        auto* output = cv::Mat::getStdAllocator()->allocate(dims, sizes, type, data, step, flags, usage);
        if (!output) { return nullptr; }
        cv::UMatData* expected = nullptr;
        if (!live.compare_exchange_strong(expected, output)) { ++errors; return output; }
        ++allocations;
        bytes += output->size;
        output->currAllocator = this;
        return output;
    }
    bool allocate(cv::UMatData* data, cv::AccessFlag flags, cv::UMatUsageFlags usage) const override
    {
        ++errors; // This CPU Mat fixture never requests secondary UMat storage.
        return cv::Mat::getStdAllocator()->allocate(data, flags, usage);
    }
    void deallocate(cv::UMatData* data) const override
    {
        if (!data) { return; }
        if (live.exchange(nullptr) != data) { ++errors; }
        ++releases;
        freed_bytes += data->size;
        data->currAllocator = cv::Mat::getStdAllocator();
        cv::Mat::getStdAllocator()->deallocate(data);
    }
};
BgrAllocator bgr_allocator;
std::mutex conversion_mutex;
}
namespace cv {
static void mfs_print_cvtColor(const Mat& input, Mat& output, int code)
{
    if (conversion.active) {
        if (conversion.report[0] >= 2) {
            conversion.report[6] = 1;
            throw cv::Exception(cv::Error::StsBadArg, "unexpected conversion count", "PrintWindow probe", __FILE__, __LINE__);
        }
        const size_t index = static_cast<size_t>(conversion.report[0]++);
        conversion.report[2 + index] = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
        if (conversion.report[0] == conversion.fail_at) {
            throw cv::Exception(cv::Error::StsBadArg, "injected conversion", "PrintWindow probe", __FILE__, __LINE__);
        }
        if (conversion.fail_at != 0) { output.allocator = &bgr_allocator; }
    }
    cv::cvtColor(input, output, code);
}
}

namespace MfsPrintConversionReference { using MAA_CTRL_UNIT_NS::ScreencapBase; }
#undef MAA_CTRL_UNIT_NS_BEGIN
#undef MAA_CTRL_UNIT_NS_END
#define MAA_CTRL_UNIT_NS_BEGIN namespace MfsPrintConversionReference {
#define MAA_CTRL_UNIT_NS_END }
#define cvtColor mfs_print_cvtColor
#include MFS_UPSTREAM_PRINT_CPP
#undef cvtColor

int32_t mfs_print_conversion_reference(void* hwnd, uint32_t fail_at, MfsParityImage* image,
    uint64_t* observations, size_t capacity)
{
    if (!image || !observations || capacity < conversion.report.size() || conversion.active || fail_at < 1 || fail_at > 2) { return 1; }
    *image = {};
    std::fill_n(observations, 12, 0);
    // Reject overlap/reentry without waiting on the window owner's callbacks.
    // The module-owned allocator outlives every successful fault observation.
    std::unique_lock lock(conversion_mutex, std::try_to_lock);
    if (!lock.owns_lock()) { return 1; }
    // OpenCV's standard allocator may be initialized lazily. Seal that setup
    // allocation/exception boundary too, before any active probe state exists.
    try { if (!bgr_allocator.reset()) { return 4; } }
    catch (const std::bad_alloc&) { return 3; }
    catch (...) { return 2; }
    conversion = {};
    conversion.active = true;
    conversion.fail_at = fail_at;
    conversion.report[1] = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
    int32_t status = 1;
    try {
        MfsPrintConversionReference::PrintWindowScreencap capture(static_cast<HWND>(hwnd));
        auto pixels = capture.screencap();
        // A missing popup may leave the requested fault unreached. Destroy its
        // local result here; no probe allocator can escape in an image owner.
        status = pixels ? 4 : 1;
    }
    catch (const std::bad_alloc&) { status = 3; }
    catch (...) { status = 2; }
    conversion.report[4] = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
    conversion.report[5] = mfs_gdi_parity_image_owners();
    conversion.report[7] = bgr_allocator.allocations.load();
    conversion.report[8] = bgr_allocator.releases.load();
    conversion.report[9] = bgr_allocator.bytes.load();
    conversion.report[10] = bgr_allocator.freed_bytes.load();
    conversion.report[11] = (bgr_allocator.errors.load() ? 1u : 0u) | (bgr_allocator.live.load() ? 2u : 0u);
    std::copy(conversion.report.begin(), conversion.report.end(), observations);
    // An unexpected outstanding allocation keeps the observation state alive
    // and rejects another call; it is not reset into a false cleanup success.
    if (!bgr_allocator.live.load()) { conversion = {}; }
    return status;
}
