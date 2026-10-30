// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Forced include for a SEPARATE diagnostic executable only. The fixed source
// bytes, native semaphore arguments/results and five-second wait remain.
// Callback wrapping and IO affect timing; this is never a parity oracle.
#include <dispatch/dispatch.h>
#include <array>
#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstdint>
#include <pthread.h>
#include "capture-observer.hpp"
namespace MfsMacObserve {
struct Slot {
    std::atomic<dispatch_semaphore_t> semaphore{nullptr};
    std::atomic<unsigned> signals{0};
    std::atomic<bool> failed_wait{false};
};
inline std::array<Slot,16> slots;
inline std::atomic<unsigned> created{0};
inline void event(const char* name,unsigned slot,long value,const char* message="") noexcept {
    uint64_t thread=0;const auto thread_error=pthread_threadid_np(nullptr,&thread);
    const auto now=std::chrono::duration_cast<std::chrono::nanoseconds>(
        std::chrono::steady_clock::now().time_since_epoch()).count();
    flockfile(stderr);
    std::fprintf(stderr,"dispatch_observer event=%s slot=%u value=%ld monotonic_ns=%lld thread=%llu thread_error=%d message=%s\n",
        name,slot,value,static_cast<long long>(now),static_cast<unsigned long long>(thread),thread_error,message);
    funlockfile(stderr);
}
inline unsigned find(dispatch_semaphore_t semaphore) noexcept {
    for(unsigned i=0;i<slots.size();++i) {
        if(slots[i].semaphore.load(std::memory_order_acquire)==semaphore) {return i;}
    }
    return static_cast<unsigned>(slots.size());
}
inline dispatch_semaphore_t create(long value) {
    auto semaphore=dispatch_semaphore_create(value);const auto index=created.fetch_add(1);
    if(index<slots.size()) {slots[index].semaphore.store(semaphore,std::memory_order_release);}
    event("create",index,value,semaphore ? "native_create_returned" : "native_create_null");return semaphore;
}
inline long wait(dispatch_semaphore_t semaphore,dispatch_time_t deadline) {
    const auto index=find(semaphore);event("wait_enter",index,0);
    const auto result=dispatch_semaphore_wait(semaphore,deadline);
    if(index<slots.size()) {slots[index].failed_wait.store(result!=0,std::memory_order_release);}
    event("wait_return",index,result);return result;
}
inline long signal(dispatch_semaphore_t semaphore) {
    const auto index=find(semaphore);event("signal_enter",index,0);
    const auto result=dispatch_semaphore_signal(semaphore);
    if(index<slots.size()) {slots[index].signals.fetch_add(1,std::memory_order_release);}
    event("signal_return",index,result);return result;
}
inline unsigned unsignalled_failed_waits() noexcept {
    unsigned pending=0;
    for(const auto& slot:slots) {
        if(slot.failed_wait.load(std::memory_order_acquire) && slot.signals.load(std::memory_order_acquire)==0) {++pending;}
    }
    return pending;
}
template<class Pump> void observe_late(Pump pump) {
    // Fixture observation AFTER the original caller has already failed. No
    // second capture, changed deadline, native cancellation or residual reclaim.
    const auto until=std::chrono::steady_clock::now()+std::chrono::seconds(15);
    event("late_observation_begin",16,unsignalled_failed_waits());
    while(unsignalled_failed_waits()!=0 && std::chrono::steady_clock::now()<until) {pump();}
    event("late_observation_end",16,unsignalled_failed_waits(),"original_failure_preserved_no_owner_drain_claim");
}
struct ErrorLog {
    const ErrorLog& operator<<(const char* message) const noexcept {
        event("source_error",16,0,message);return *this;
    }
};
inline const ErrorLog errors;
}
// Only these original source call sites are substituted at compilation. The
// wrappers above call the real native symbols before these macros exist.
#define dispatch_semaphore_create MfsMacObserve::create
#define dispatch_semaphore_wait MfsMacObserve::wait
#define dispatch_semaphore_signal MfsMacObserve::signal
