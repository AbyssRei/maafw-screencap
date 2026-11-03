// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "native-sample-state.hpp"
#include "live-sample-hook.hpp"
#include <cstdint>
#include <filesystem>
#import <ScreenCaptureKit/ScreenCaptureKit.h>

namespace MfsMacSample {
struct Observation;
using Scope=ThreadScope<Observation>;
std::shared_ptr<Observation> begin(uint32_t call,std::shared_ptr<LiveHook> hook={});
// Called after the real source return has already been saved. The extra wait
// observes only this wrapper's post-forward copy, never replaces/retries the
// source return and never proves framework callback unregistration/drainage.
void write(const std::shared_ptr<Observation>&,const std::filesystem::path&);
}
@interface SCShareableContent (MfsNativeSampleObservation)
+ (void)mfs_native_sample_content:(void (^)(SCShareableContent*,NSError*))completion;
@end
@interface SCScreenshotManager (MfsNativeSampleObservation)
+ (void)mfs_native_sample_capture:(SCContentFilter*)filter configuration:(SCStreamConfiguration*)configuration
    completionHandler:(void (^)(CMSampleBufferRef,NSError*))completion;
@end
// Set ONLY on the separate original-class object target. Scene and Rust native
// calls keep their real selectors; nothing is swizzled or globally replaced.
#ifdef MFS_MAC_NATIVE_SAMPLE_ALIAS
#define getShareableContentWithCompletionHandler mfs_native_sample_content
#define captureSampleBufferWithFilter mfs_native_sample_capture
#endif
