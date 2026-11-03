// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Independent diagnostic copy at a real native completion, never an oracle
// substitute, screenshot retry, pixel mask or capture implementation.
#include "native-sample-observer.hpp"
#include "same-binary-scene.h"
#include <CoreMedia/CoreMedia.h>
#include <CoreVideo/CoreVideo.h>
#include <CoreGraphics/CoreGraphics.h>
#include <atomic>
#include <chrono>
#include <climits>
#include <cstddef>
#include <cstring>
#include <fstream>
#include <limits>
#include <pthread.h>
#include <stdexcept>
#include <thread>
#include <unistd.h>
#include <vector>

namespace MfsMacSample {
namespace {
uint64_t thread_id() noexcept {uint64_t value=0;return pthread_threadid_np(nullptr,&value)==0 ? value : 0;}
struct Snapshot {
    size_t width=0,height=0,stride=0,data_size=0,row=0,span=0;
    uint32_t format=0;
    int planar=0,lock_code=-1,unlock_code=-1;
    std::vector<uint8_t> bytes;
};
size_t multiply(size_t a,size_t b) {
    if(b && a>std::numeric_limits<size_t>::max()/b) {throw std::runtime_error("native sample size overflow");}
    return a*b;
}
void copy(CMSampleBufferRef sample,Snapshot& out) {
    CVImageBufferRef image=sample ? CMSampleBufferGetImageBuffer(sample) : nullptr;
    if(!image) {throw std::runtime_error("native sample has no image");}
    out.lock_code=CVPixelBufferLockBaseAddress(image,kCVPixelBufferLock_ReadOnly);
    if(out.lock_code!=kCVReturnSuccess) {throw std::runtime_error("native observation lock failed");}
    struct Unlock {
        CVImageBufferRef image;Snapshot& output;
        ~Unlock() {output.unlock_code=CVPixelBufferUnlockBaseAddress(image,kCVPixelBufferLock_ReadOnly);}
    } unlock{image,out};
    out.width=CVPixelBufferGetWidth(image);out.height=CVPixelBufferGetHeight(image);
    out.stride=CVPixelBufferGetBytesPerRow(image);out.data_size=CVPixelBufferGetDataSize(image);
    out.format=CVPixelBufferGetPixelFormatType(image);out.planar=CVPixelBufferIsPlanar(image);
    if(out.planar || out.format!=kCVPixelFormatType_32BGRA || out.width==0 || out.height==0 ||
       out.width>INT_MAX || out.height>INT_MAX) {throw std::runtime_error("native observation layout differs");}
    out.row=multiply(out.width,4);
    const auto prefix=multiply(out.height-1,out.stride);
    if(prefix>std::numeric_limits<size_t>::max()-out.row) {throw std::runtime_error("native sample span overflow");}
    out.span=prefix+out.row;
    const auto packed=multiply(out.row,out.height);
    const auto* data=static_cast<const uint8_t*>(CVPixelBufferGetBaseAddress(image));
    if(!data || out.stride<out.row || out.span>out.data_size ||
       out.span>static_cast<size_t>(std::numeric_limits<std::ptrdiff_t>::max()) ||
       packed>static_cast<size_t>(std::numeric_limits<std::ptrdiff_t>::max())) {
        throw std::runtime_error("native observation unreadable span");
    }
    out.bytes.resize(packed);
    // The successful read lock and checked full span cover every target row.
    // No native padding or pointer escapes, and the lock ends before forwarding.
    for(size_t y=0;y<out.height;++y) {std::memcpy(out.bytes.data()+y*out.row,data+y*out.stride,out.row);}
}
void phase(std::ostream& stream,const Snapshot& s) {
    stream<<"{\"width\":"<<s.width<<",\"height\":"<<s.height<<",\"stride\":"<<s.stride
          <<",\"data_size\":"<<s.data_size<<",\"row\":"<<s.row<<",\"span\":"<<s.span
          <<",\"format\":"<<s.format<<",\"planar\":"<<s.planar<<",\"lock_code\":"<<s.lock_code
          <<",\"unlock_code\":"<<s.unlock_code<<",\"packed_bytes\":"<<s.bytes.size()<<'}';
}
void pixels(const std::filesystem::path& path,const std::vector<uint8_t>& data) {
    if(data.size()>static_cast<size_t>(std::numeric_limits<std::streamsize>::max())) {throw std::runtime_error("native sample file narrowing");}
    std::ofstream stream(path,std::ios::binary);stream.exceptions(std::ios::badbit|std::ios::failbit);
    if(!data.empty()) {stream.write(reinterpret_cast<const char*>(data.data()),static_cast<std::streamsize>(data.size()));}
    stream.close();
}
std::atomic<uint64_t> serial{1};
}
struct Observation {
    std::shared_ptr<LiveHook> hook;
    uint32_t call;uint64_t id,caller=thread_id();
    std::atomic<uint64_t> content_enter{0},content_callback{0},sample_enter{0},sample_callback{0};
    std::atomic<uint64_t> content_enter_thread{0},content_callback_thread{0},sample_enter_thread{0},sample_callback_thread{0};
    std::atomic<uint32_t> fault{0},forwarded{0};
    std::atomic<uint64_t> display_count{0},display_id{0},display_width{0},display_height{0};
    std::atomic<uint64_t> config_width{0},config_height{0},config_format{0};
    std::atomic<uint32_t> content_present{0},content_error_present{0},filter_present{0},config_present{0},config_srgb{0},config_cursor{0};
    std::atomic<bool> ready{false};
    bool sample_present=false,error_present=false;
    Snapshot before,after;
};
std::shared_ptr<Observation> begin(uint32_t call,std::shared_ptr<LiveHook> hook) {
    if(call>=3 || Scope::current()) {throw std::runtime_error("invalid native sample observation association");}
    auto state=std::make_shared<Observation>();state->hook=std::move(hook);state->call=call;state->id=serial.fetch_add(1);
    if(state->id==0 || state->id==UINT64_MAX || state->caller==0) {throw std::runtime_error("native sample identity overflow");}
    return state;
}
void write(const std::shared_ptr<Observation>& state,const std::filesystem::path& directory) {
    const auto until=std::chrono::steady_clock::now()+std::chrono::seconds(2);
    while(!state->ready.load(std::memory_order_acquire) && std::chrono::steady_clock::now()<until) {
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    const bool ready=state->ready.load(std::memory_order_acquire);
    const auto stem="original-native-"+std::to_string(state->call);
    std::ofstream out(directory/(stem+".json"));out.exceptions(std::ios::badbit|std::ios::failbit);
    out<<"{\"schema\":1,\"call\":"<<state->call<<",\"serial\":"<<state->id<<",\"pid\":"<<getpid()
       <<",\"caller_thread\":"<<state->caller<<",\"ready\":"<<ready<<",\"fault\":"<<state->fault.load()
       <<",\"content_entries\":"<<state->content_enter.load()<<",\"content_callbacks\":"<<state->content_callback.load()
       <<",\"sample_entries\":"<<state->sample_enter.load()<<",\"sample_callbacks\":"<<state->sample_callback.load()
       <<",\"content_enter_thread\":"<<state->content_enter_thread.load()<<",\"content_callback_thread\":"<<state->content_callback_thread.load()
       <<",\"sample_enter_thread\":"<<state->sample_enter_thread.load()<<",\"sample_callback_thread\":"<<state->sample_callback_thread.load()
       <<",\"content_present\":"<<state->content_present.load()<<",\"content_error_present\":"<<state->content_error_present.load()
       <<",\"display_count\":"<<state->display_count.load()<<",\"display_id\":"<<state->display_id.load()
       <<",\"display_width\":"<<state->display_width.load()<<",\"display_height\":"<<state->display_height.load()
       <<",\"filter_present\":"<<state->filter_present.load()<<",\"config_present\":"<<state->config_present.load()
       <<",\"config_width\":"<<state->config_width.load()<<",\"config_height\":"<<state->config_height.load()
       <<",\"config_format\":"<<state->config_format.load()<<",\"config_srgb\":"<<state->config_srgb.load()
       <<",\"config_cursor\":"<<state->config_cursor.load()
       <<",\"forwarded\":"<<state->forwarded.load()<<",\"sample_present\":"<<(ready && state->sample_present)
       <<",\"error_present\":"<<(ready && state->error_present)<<",\"before\":";
    // Never race an unfinished callback by reading its vector or ordinary fields.
    const Snapshot empty;
    phase(out,ready ? state->before : empty);out<<",\"after\":";phase(out,ready ? state->after : empty);out<<"}\n";out.close();
    if(ready) {
        pixels(directory/(stem+"-before.bgra"),state->before.bytes);
        pixels(directory/(stem+"-after.bgra"),state->after.bytes);
    }
    if(!ready || state->fault.load()!=0) {throw std::runtime_error("incomplete native sample observation; original result retained");}
}
}

@implementation SCShareableContent (MfsNativeSampleObservation)
+ (void)mfs_native_sample_content:(void (^)(SCShareableContent*,NSError*))completion {
    const auto state=MfsMacSample::Scope::current();
    if(state) {state->content_enter.fetch_add(1);state->content_enter_thread.store(MfsMacSample::thread_id());}
    // The escaping block owns state independently from the original caller.
    [SCShareableContent getShareableContentWithCompletionHandler:^(SCShareableContent* content,NSError* error) {
        if(state) {
            state->content_callback.fetch_add(1);state->content_callback_thread.store(MfsMacSample::thread_id());
            state->content_present.store(content!=nil);state->content_error_present.store(error!=nil);
            state->display_count.store(content.displays.count);
            SCDisplay* first=content.displays.count ? content.displays[0] : nil;
            state->display_id.store(first ? first.displayID : 0);
            state->display_width.store(first ? first.width : 0);state->display_height.store(first ? first.height : 0);
        }
        MfsMacSample::Scope scope(state);completion(content,error);
    }];
}
@end
@implementation SCScreenshotManager (MfsNativeSampleObservation)
+ (void)mfs_native_sample_capture:(SCContentFilter*)filter configuration:(SCStreamConfiguration*)configuration
    completionHandler:(void (^)(CMSampleBufferRef,NSError*))completion {
    const auto state=MfsMacSample::Scope::current();
    if(state) {
        state->sample_enter.fetch_add(1);state->sample_enter_thread.store(MfsMacSample::thread_id());
        state->filter_present.store(filter!=nil);state->config_present.store(configuration!=nil);
        state->config_width.store(configuration.width);state->config_height.store(configuration.height);
        state->config_format.store(configuration.pixelFormat);state->config_cursor.store(configuration.showsCursor!=NO);
        const auto space=configuration.colorSpaceName;
        state->config_srgb.store(space && CFEqual(space,kCGColorSpaceSRGB));
    }
    [SCScreenshotManager captureSampleBufferWithFilter:filter configuration:configuration
        completionHandler:^(CMSampleBufferRef sample,NSError* error) {
            const bool first=state && state->sample_callback.fetch_add(1)==0;
            if(state && !first) {state->fault.fetch_or(4);}
            if(first) {
                state->sample_callback_thread.store(MfsMacSample::thread_id());
                state->sample_present=sample!=nullptr;state->error_present=error!=nil;
                try {MfsMacSample::copy(sample,state->before);}catch(...) {state->fault.fetch_or(1);}
            }
            // Exactly the original sample/error and native channel, once. The
            // extra CPU copies/locks intentionally affect this diagnostic's timing.
            completion(sample,error);
            if(first) {
                state->forwarded.fetch_add(1);
                // Separate opt-in same-sample diagnostic. Original conversion,
                // release and signal have already run. This synchronous borrow
                // cannot prolong a native pixel pointer beyond the callback;
                // only independently owned CPU results may survive it.
                if(state->hook && state->hook->visit(state->hook->context,sample,state->call)!=0) {
                    state->fault.fetch_or(8);
                }
                try {MfsMacSample::copy(sample,state->after);}catch(...) {state->fault.fetch_or(2);}
                state->ready.store(true,std::memory_order_release);
            }
        }];
}
@end
uint32_t mfs_mac_native_sample_observer_version(void) noexcept {return 1;}
