// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "capture-observer.hpp"
// The forwarding implementation calls the real native selectors. Other
// translation units in this diagnostic target retain their compile-time aliases.
#undef getShareableContentWithCompletionHandler
#undef captureSampleBufferWithFilter
#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstdint>
#include <cstring>
#include <pthread.h>
#include "input-weak-observer.hpp"

namespace {
std::atomic<uint64_t> next_request{1};
MfsMacObserve::CallbackLedger callbacks;
MfsMacObserve::CallbackLedger block_leases;
MfsMacObserve::InputWeakRegistry& input_weak_registry() {
    // Bounded process-lifetime diagnostic metadata: at most64 weak tables.
    // Neither tables nor registry own filter/config objects. Do not tear this
    // registry down while the native framework may still call the observer.
    static auto* registry=new MfsMacObserve::InputWeakRegistry;
    return *registry;
}
void input_weak_line(const char* phase,const MfsMacObserve::InputWeakSnapshot& value) noexcept {
    std::fprintf(stderr,"capture_input_weak schema=1 phase=%s request=%llu registered_filter=%llu registered_config=%llu filter=%llu config=%llu begin_ns=%llu end_ns=%llu thread=%llu thread_error=%d valid=%u\n",
        phase,static_cast<unsigned long long>(value.request),
        static_cast<unsigned long long>(value.registered_filter),static_cast<unsigned long long>(value.registered_config),
        static_cast<unsigned long long>(value.filter),static_cast<unsigned long long>(value.config),
        static_cast<unsigned long long>(value.begin_ns),static_cast<unsigned long long>(value.end_ns),
        static_cast<unsigned long long>(value.thread),value.thread_error,static_cast<unsigned>(value.valid));
}
struct Evidence {
    bool object=false,error=false;
    int64_t code=0;
    uint64_t displays=0,windows=0,display=0,width=0,height=0;
};
void stage(const char* name,uint64_t request,Evidence evidence={}) noexcept {
    uint64_t thread=0;const auto thread_error=pthread_threadid_np(nullptr,&thread);
    const auto now=std::chrono::duration_cast<std::chrono::nanoseconds>(
        std::chrono::steady_clock::now().time_since_epoch()).count();
    flockfile(stderr);
    // Serialize the transition and its existing stage line with late snapshots.
    // Native submission/handler execution stay outside this short stdio lock.
    if(std::strcmp(name,"content_enter")==0 || std::strcmp(name,"sample_enter")==0) {(void)callbacks.submit();}
    else if(std::strcmp(name,"content_forwarded")==0 || std::strcmp(name,"sample_forwarded")==0) {(void)callbacks.forward();}
    std::fprintf(stderr,"capture_observer stage=%s request=%llu monotonic_ns=%lld thread=%llu thread_error=%d object=%u error=%u error_code=%lld displays=%llu windows=%llu display=%llu width=%llu height=%llu\n",
        name,static_cast<unsigned long long>(request),static_cast<long long>(now),static_cast<unsigned long long>(thread),thread_error,
        static_cast<unsigned>(evidence.object),static_cast<unsigned>(evidence.error),static_cast<long long>(evidence.code),
        static_cast<unsigned long long>(evidence.displays),static_cast<unsigned long long>(evidence.windows),static_cast<unsigned long long>(evidence.display),
        static_cast<unsigned long long>(evidence.width),static_cast<unsigned long long>(evidence.height));
    funlockfile(stderr);
}
}

MfsMacObserve::CallbackCounts mfs_mac_pending_capture_callbacks() noexcept {return callbacks.snapshot();}
MfsMacObserve::CallbackCounts mfs_mac_capture_block_leases() noexcept {return block_leases.snapshot();}
void mfs_mac_capture_late_snapshot(const char* phase,unsigned semaphore_pending) noexcept {
    // Weak Foundation reads may temporarily retain their returned objects.
    // Complete/drain them before taking stderr's existing transition lock.
    const auto weak=input_weak_registry().snapshot();
    uint64_t thread=0;const auto thread_error=pthread_threadid_np(nullptr,&thread);
    flockfile(stderr);
    const auto value=callbacks.snapshot();
    const auto now=std::chrono::duration_cast<std::chrono::nanoseconds>(
        std::chrono::steady_clock::now().time_since_epoch()).count();
    std::fprintf(stderr,"capture_pending schema=1 phase=%s monotonic_ns=%lld thread=%llu thread_error=%d submitted=%u forwarded=%u pending=%u valid=%u semaphore_pending=%u budget_ns=15000000000\n",
        phase,static_cast<long long>(now),static_cast<unsigned long long>(thread),thread_error,
        value.submitted,value.forwarded,value.pending,static_cast<unsigned>(value.valid),semaphore_pending);
    // This separate atomic snapshot is not simultaneous with callback/semaphore
    // counts. It observes only C++ token copies captured by this wrapper block.
    const auto leases=block_leases.snapshot();
    const auto lease_now=std::chrono::duration_cast<std::chrono::nanoseconds>(
        std::chrono::steady_clock::now().time_since_epoch()).count();
    std::fprintf(stderr,"capture_block_leases schema=1 phase=%s monotonic_ns=%lld thread=%llu thread_error=%d acquired=%u released=%u live=%u valid=%u budget_ns=15000000000\n",
        phase,static_cast<long long>(lease_now),static_cast<unsigned long long>(thread),thread_error,
        leases.submitted,leases.forwarded,leases.pending,static_cast<unsigned>(leases.valid));
    std::fprintf(stderr,"capture_input_weak_summary schema=1 phase=%s registered=%u valid=%u capacity=32 begin_ns=%llu end_ns=%llu thread=%llu thread_error=%d\n",
        phase,weak.count,static_cast<unsigned>(weak.valid),static_cast<unsigned long long>(weak.begin_ns),
        static_cast<unsigned long long>(weak.end_ns),static_cast<unsigned long long>(weak.thread),weak.thread_error);
    for(unsigned i=0;i<weak.count;++i) {input_weak_line(phase,weak.rows[i]);}
    funlockfile(stderr);
}

@implementation SCShareableContent (MfsCaptureObservation)
+ (void)mfs_observe_content:(void (^)(SCShareableContent*,NSError*))completion {
    const auto request=next_request.fetch_add(1,std::memory_order_relaxed);
    stage("content_enter",request);
    const MfsMacObserve::BlockLease lease(block_leases);
    // Copying the native escaping block also copies its captured block under
    // the Clang Blocks ABI. No callback/context is borrowed from this stack,
    // explicitly released early, rescheduled, or retained beyond that wrapper.
    [SCShareableContent getShareableContentWithCompletionHandler:^(SCShareableContent* content,NSError* error) {
        lease.touch();
        SCDisplay* first=content.displays.count ? content.displays[0] : nil;
        const Evidence evidence{content!=nil,error!=nil,static_cast<int64_t>(error.code),
            static_cast<uint64_t>(content.displays.count),static_cast<uint64_t>(content.windows.count),
            first ? static_cast<uint64_t>(first.displayID) : 0,
            first ? static_cast<uint64_t>(first.width) : 0,first ? static_cast<uint64_t>(first.height) : 0};
        stage("content_callback",request,evidence);
        completion(content,error);
        stage("content_forwarded",request,evidence);
    }];
}
@end

@implementation SCScreenshotManager (MfsCaptureObservation)
+ (void)mfs_observe_sample:(SCContentFilter*)filter configuration:(SCStreamConfiguration*)configuration
    completionHandler:(void (^)(CMSampleBufferRef,NSError*))completion {
    const auto request=next_request.fetch_add(1,std::memory_order_relaxed);
    // Read the original configured extents before forwarding. The wrapper
    // block captures these values, request ID, received completion and the
    // process-lifetime diagnostic token, never additional filter/config/sample
    // owners or a borrowed stack object.
    const auto width=static_cast<uint64_t>(configuration.width),height=static_cast<uint64_t>(configuration.height);
    stage("sample_enter",request,{filter!=nil && configuration!=nil,false,0,0,0,0,width,height});
    const auto weak=input_weak_registry().bind(request,filter,configuration);
    flockfile(stderr);input_weak_line("registered",weak);funlockfile(stderr);
    const MfsMacObserve::BlockLease lease(block_leases);
    [SCScreenshotManager captureSampleBufferWithFilter:filter configuration:configuration
        completionHandler:^(CMSampleBufferRef sample,NSError* error) {
            lease.touch();
            const Evidence evidence{sample!=nullptr,error!=nil,static_cast<int64_t>(error.code),0,0,0,width,height};
            stage("sample_callback",request,evidence);
            completion(sample,error);
            stage("sample_forwarded",request,evidence);
        }];
}
@end
