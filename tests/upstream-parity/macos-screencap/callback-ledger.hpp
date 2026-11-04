// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <atomic>
#include <cstdint>
#include <limits>

namespace MfsMacObserve {
struct CallbackCounts {
    uint32_t submitted=0,forwarded=0,pending=0;
    bool valid=true;
};
// Diagnostic submission/handler-return counts only. Neither a returned handler
// nor a zero pending count observes native block disposal or cancellation.
// One atomic word prevents a reader combining counts from different updates.
class CallbackLedger {
    std::atomic<uint64_t> counts_;
    std::atomic<bool> fault_;
    static constexpr uint64_t pack(uint32_t submitted,uint32_t forwarded) noexcept {
        return static_cast<uint64_t>(submitted)|(static_cast<uint64_t>(forwarded)<<32);
    }
public:
    explicit CallbackLedger(uint32_t submitted=0,uint32_t forwarded=0) noexcept
        : counts_(pack(submitted,forwarded)),fault_(forwarded>submitted) {}
    bool submit() noexcept {
        auto current=counts_.load(std::memory_order_acquire);
        for(;;) {
            const auto submitted=static_cast<uint32_t>(current);
            const auto forwarded=static_cast<uint32_t>(current>>32);
            if(submitted==std::numeric_limits<uint32_t>::max() || forwarded>submitted) {
                fault_.store(true,std::memory_order_release);return false;
            }
            if(counts_.compare_exchange_weak(current,pack(submitted+1,forwarded),
                std::memory_order_acq_rel,std::memory_order_acquire)) {return true;}
        }
    }
    bool forward() noexcept {
        auto current=counts_.load(std::memory_order_acquire);
        for(;;) {
            const auto submitted=static_cast<uint32_t>(current);
            const auto forwarded=static_cast<uint32_t>(current>>32);
            if(forwarded>=submitted) {fault_.store(true,std::memory_order_release);return false;}
            if(counts_.compare_exchange_weak(current,pack(submitted,forwarded+1),
                std::memory_order_acq_rel,std::memory_order_acquire)) {return true;}
        }
    }
    CallbackCounts snapshot() const noexcept {
        const auto current=counts_.load(std::memory_order_acquire);
        const auto submitted=static_cast<uint32_t>(current);
        const auto forwarded=static_cast<uint32_t>(current>>32);
        const bool valid=forwarded<=submitted && !fault_.load(std::memory_order_acquire);
        return {submitted,forwarded,forwarded<=submitted ? submitted-forwarded : 0,valid};
    }
};

// Diagnostic C++ capture only. Its ledger must outlive every copy, including
// copies in native heap blocks. The observer uses a process-lifetime ledger.
// Counts include the local token and stack-block copies; they are not a native
// block retain count, a callback-return count or a whole-session drain proof.
class BlockLease {
    CallbackLedger* ledger_;
    bool registered_;
public:
    explicit BlockLease(CallbackLedger& ledger) noexcept
        : ledger_(&ledger),registered_(ledger_->submit()) {}
    BlockLease(const BlockLease& other) noexcept
        : ledger_(other.ledger_),registered_(ledger_->submit()) {}
    BlockLease& operator=(const BlockLease&)=delete;
    ~BlockLease() noexcept {if(registered_) {(void)ledger_->forward();}}
    bool registered() const noexcept {return registered_;}
    void touch() const noexcept {(void)ledger_->snapshot();}
};
}
