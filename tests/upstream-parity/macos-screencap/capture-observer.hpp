// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#import <ScreenCaptureKit/ScreenCaptureKit.h>

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
