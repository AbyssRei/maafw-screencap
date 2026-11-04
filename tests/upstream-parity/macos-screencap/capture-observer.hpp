// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#import <ScreenCaptureKit/ScreenCaptureKit.h>
#include "callback-ledger.hpp"

// Separate diagnostic query and timestamped snapshots. Pending includes the
// fixture's metadata requests, even before an original semaphore is created.
MfsMacObserve::CallbackCounts mfs_mac_pending_capture_callbacks() noexcept;
MfsMacObserve::CallbackCounts mfs_mac_capture_block_leases() noexcept;
void mfs_mac_capture_late_snapshot(const char* phase,unsigned semaphore_pending) noexcept;

// Only the separate observer target imports these selectors. No method is
// swizzled or replaced at runtime. The category preserves filter/config and
// callback values, invoking the received completion on the native channel.
@interface SCShareableContent (MfsCaptureObservation)
+ (void)mfs_observe_content:(void (^)(SCShareableContent*,NSError*))completion;
@end
@interface SCScreenshotManager (MfsCaptureObservation)
+ (void)mfs_observe_sample:(SCContentFilter*)filter configuration:(SCStreamConfiguration*)configuration
    completionHandler:(void (^)(CMSampleBufferRef,NSError*))completion;
@end

#define getShareableContentWithCompletionHandler mfs_observe_content
#define captureSampleBufferWithFilter mfs_observe_sample
