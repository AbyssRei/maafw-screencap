// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#import <Foundation/Foundation.h>
#include <array>
#include <chrono>
#include <cstdint>
#include <mutex>
#include <pthread.h>

namespace MfsMacObserve {
struct InputWeakSnapshot {
    uint64_t request=0,registered_filter=0,registered_config=0,filter=0,config=0;
    uint64_t begin_ns=0,end_ns=0,thread=0;
    int thread_error=0;
    bool valid=true;
};
inline uint64_t weak_clock() noexcept {
    return static_cast<uint64_t>(std::chrono::duration_cast<std::chrono::nanoseconds>(
        std::chrono::steady_clock::now().time_since_epoch()).count());
}
class InputWeakPair {
    NSHashTable* filter_=nil;
    NSHashTable* config_=nil;
    uint64_t request_=0,filter_pointer_=0,config_pointer_=0;
    bool valid_=true;
public:
    InputWeakPair()=default;
    InputWeakPair(const InputWeakPair&)=delete;
    InputWeakPair& operator=(const InputWeakPair&)=delete;
    ~InputWeakPair() {[filter_ release];[config_ release];}
    void bind(uint64_t request,id filter,id config) {
        if(request_ || !request || !filter || !config) {valid_=false;return;}
        request_=request;
        filter_pointer_=reinterpret_cast<uintptr_t>(filter);
        config_pointer_=reinterpret_cast<uintptr_t>(config);
        const auto options=static_cast<NSPointerFunctionsOptions>(NSHashTableWeakMemory|NSHashTableObjectPointerPersonality);
        // Registration can also create autoreleased temporary owners inside
        // Foundation. Drain those before returning to the capture caller, as
        // snapshot() does for its reads; the alloc/init tables stay owned here.
        @autoreleasepool {
            filter_=[[NSHashTable alloc] initWithOptions:options capacity:1];
            config_=[[NSHashTable alloc] initWithOptions:options capacity:1];
            valid_=filter_!=nil && config_!=nil;
            [filter_ addObject:filter];[config_ addObject:config];
        }
    }
    InputWeakSnapshot snapshot() const {
        InputWeakSnapshot result;
        result.request=request_;result.registered_filter=filter_pointer_;
        result.registered_config=config_pointer_;result.valid=valid_;
        result.thread_error=pthread_threadid_np(nullptr,&result.thread);
        result.begin_ns=weak_clock();
        // allObjects creates temporary strong snapshot references. Drain this
        // private pool before returning scalar evidence; no native input owner
        // escapes. The interval is not an atomic liveness/callback snapshot.
        @autoreleasepool {
            NSArray* filters=[filter_ allObjects];NSArray* configs=[config_ allObjects];
            if(filters.count>1 || configs.count>1) {result.valid=false;}
            result.filter=filters.count ? reinterpret_cast<uintptr_t>(filters[0]) : 0;
            result.config=configs.count ? reinterpret_cast<uintptr_t>(configs[0]) : 0;
            if((result.filter && result.filter!=filter_pointer_)
                || (result.config && result.config!=config_pointer_)) {result.valid=false;}
        }
        result.end_ns=weak_clock();return result;
    }
};
struct InputWeakBatch {
    std::array<InputWeakSnapshot,32> rows;
    unsigned count=0;
    uint64_t begin_ns=0,end_ns=0,thread=0;
    int thread_error=0;
    bool valid=true;
};
class InputWeakRegistry {
    std::mutex mutex_;
    std::array<InputWeakPair,32> pairs_;
    unsigned count_=0;
    bool valid_=true;
public:
    InputWeakSnapshot bind(uint64_t request,id filter,id config) {
        const std::lock_guard lock(mutex_);
        if(count_>=pairs_.size()) {valid_=false;InputWeakSnapshot value;value.valid=false;return value;}
        auto& pair=pairs_[count_++];pair.bind(request,filter,config);
        const auto result=pair.snapshot();valid_=valid_ && result.valid;return result;
    }
    InputWeakBatch snapshot() {
        const std::lock_guard lock(mutex_);
        InputWeakBatch result;result.count=count_;result.valid=valid_;
        result.thread_error=pthread_threadid_np(nullptr,&result.thread);result.begin_ns=weak_clock();
        for(unsigned i=0;i<count_;++i) {result.rows[i]=pairs_[i].snapshot();result.valid=result.valid && result.rows[i].valid;}
        result.end_ns=weak_clock();
        return result;
    }
};
}
