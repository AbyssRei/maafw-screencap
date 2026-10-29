// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "Screencap/ScreenCaptureKitScreencap.h"
#include "MaaFramework/MaaDef.h"
#include <atomic>
#include <memory>
#include <stdexcept>
#include <string>
namespace MfsMacManagerNative {
using ScreencapBase=MfsOriginalMac::ScreencapBase;
inline std::atomic<uint64_t> creates=0,calls=0,drops=0,input_attempts=0;
struct ScreenCaptureKitScreencap : MfsOriginalMac::ScreenCaptureKitScreencap {
    explicit ScreenCaptureKitScreencap(uint32_t id) : MfsOriginalMac::ScreenCaptureKitScreencap(id) { ++creates; }
    ~ScreenCaptureKitScreencap() override { ++drops; }
    std::optional<cv::Mat> screencap() override {
        ++calls;
        return MfsOriginalMac::ScreenCaptureKitScreencap::screencap();
    }
};
inline void mfs_mac_input_attempt() { ++input_attempts; }
#include "manager-shim.h"
#define LogWarn mfs_mac_null_log
#define LogError mfs_mac_null_log
#include "macos-manager-source.inc"
#undef LogError
#undef LogWarn
}
