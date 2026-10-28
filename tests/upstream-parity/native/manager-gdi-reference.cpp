// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "manager-script.h"
#include "gdi-probe-internal.hpp"
#include "Screencap/GdiScreencap.h"
#include "MaaUtils/SafeWindows.hpp"
#include <chrono>
#include <memory>
#include <stdexcept>
#include <unordered_map>
#include <opencv2/core.hpp>

namespace MfsManagerGdiSource {
using MAA_CTRL_UNIT_NS::ScreencapBase;
thread_local MfsManagerGdiReport* active=nullptr;
struct GdiScreencap : MAA_CTRL_UNIT_NS::GdiScreencap {
    using MAA_CTRL_UNIT_NS::GdiScreencap::GdiScreencap;
    std::optional<cv::Mat> screencap() override {
        const auto index=active->calls++;
        if(index>=3) { throw std::logic_error("unexpected manager GDI call count"); }
        active->threads[index]=GetCurrentThreadId();
        active->dpi_v2[index]=AreDpiAwarenessContextsEqual(GetThreadDpiAwarenessContext(),DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        return MAA_CTRL_UNIT_NS::GdiScreencap::screencap();
    }
};
struct Unavailable : ScreencapBase {
    explicit Unavailable(HWND) { throw std::logic_error("outside GDI-only manager reference"); }
    std::optional<cv::Mat> screencap() override { return std::nullopt; }
};
using FramePoolWithPseudoMinimizeScreencap=Unavailable;
using DesktopDupScreencap=Unavailable;
using DesktopDupWindowScreencap=Unavailable;
using PrintWindowWithPseudoMinimizeScreencap=Unavailable;
using ScreenDCScreencap=Unavailable;
#include "manager-shim.h"
struct NullLog { template<class T> const NullLog& operator<<(const T&) const noexcept { return *this; } } null_log;
#define LogInfo null_log
#define LogWarn null_log
#define LogError null_log
#define LogFunc do {} while(false)
#define VAR(x) x
#define VAR_VOIDP(x) x
#include "manager-time.inc"
#include "manager-source.inc"
#undef VAR_VOIDP
#undef VAR
#undef LogFunc
#undef LogError
#undef LogWarn
#undef LogInfo
}
int32_t mfs_manager_gdi_reference(void* window,MfsParityImage* output,MfsManagerGdiReport* report) {
    if(!output || !report) { return 1; }
    *output={}; *report={};
    if(MfsManagerGdiSource::active) { return 1; }
    MfsManagerGdiSource::active=report;
    struct Reset { ~Reset() { MfsManagerGdiSource::active=nullptr; } } reset;
    try {
        cv::Mat image;
        {
            MfsManagerGdiSource::Win32ControlUnitMgr manager(static_cast<HWND>(window),MaaWin32ScreencapMethod_GDI,MaaWin32InputMethod_None,MaaWin32InputMethod_None);
            report->connected=manager.connect();
            report->selected=manager.screencap_ ? 1 : 0;
            report->calls_after_connect=report->calls;
            if(!report->connected) { return 1; }
            report->captured=manager.screencap(image);
            manager.inactive();
            if(!report->captured) { return 1; }
        }
        return mfs_parity::transfer_image(std::move(image),output);
    } catch(const std::bad_alloc&) { return 3; } catch(...) { return 2; }
}
