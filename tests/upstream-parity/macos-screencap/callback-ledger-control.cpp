// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Portable bookkeeping control, with no Apple API, desktop or native callback.
#include "callback-ledger.hpp"
#include <array>
#include <atomic>
#include <iostream>
#include <stdexcept>
#include <thread>
namespace {
void require(bool value) {if(!value) {throw std::runtime_error("callback ledger control differs");}}
void equal(const MfsMacObserve::CallbackLedger& ledger,uint32_t submitted,uint32_t forwarded,bool valid=true) {
    const auto value=ledger.snapshot();
    require(value.submitted==submitted && value.forwarded==forwarded && value.valid==valid);
    require(value.pending==(forwarded<=submitted ? submitted-forwarded : 0));
}
}
int main() {
    try {
        using MfsMacObserve::CallbackLedger;
        CallbackLedger ordinary;
        equal(ordinary,0,0);require(ordinary.submit());equal(ordinary,1,0);
        require(ordinary.submit());require(ordinary.forward());equal(ordinary,2,1);
        require(ordinary.forward());equal(ordinary,2,2);
        require(!ordinary.forward());equal(ordinary,2,2,false);
        constexpr auto maximum=std::numeric_limits<uint32_t>::max();
        CallbackLedger last(maximum-1,maximum-1);
        require(last.submit());equal(last,maximum,maximum-1);
        require(last.forward());equal(last,maximum,maximum);
        require(!last.submit());equal(last,maximum,maximum,false);
        CallbackLedger invalid(0,1);equal(invalid,0,1,false);
        require(!invalid.submit());require(!invalid.forward());equal(invalid,0,1,false);
        CallbackLedger concurrent;
        std::atomic<bool> fault=false;
        std::array<std::thread,8> workers;
        for(auto& worker:workers) {
            worker=std::thread([&] {
                for(unsigned call=0;call<250;++call) {
                    if(!concurrent.submit()) {fault=true;}
                    std::this_thread::yield();
                    const auto observed=concurrent.snapshot();
                    if(!observed.valid || observed.forwarded>observed.submitted
                        || observed.pending!=observed.submitted-observed.forwarded) {fault=true;}
                    if(!concurrent.forward()) {fault=true;}
                }
            });
        }
        for(auto& worker:workers) {worker.join();}
        require(!fault.load());equal(concurrent,2000,2000);
        CallbackLedger owners;
        {
            MfsMacObserve::BlockLease first(owners);require(first.registered());equal(owners,1,0);
            {MfsMacObserve::BlockLease second(first);second.touch();equal(owners,2,0);}
            equal(owners,2,1);
        }
        equal(owners,2,2);
        CallbackLedger exhausted(maximum,maximum);
        {MfsMacObserve::BlockLease refused(exhausted);require(!refused.registered());}
        equal(exhausted,maximum,maximum,false);
        CallbackLedger copied;
        for(auto& worker:workers) {
            worker=std::thread([&] {
                for(unsigned call=0;call<250;++call) {
                    MfsMacObserve::BlockLease first(copied),second(first);
                    first.touch();second.touch();
                    if(!first.registered() || !second.registered() || !copied.snapshot().valid) {fault=true;}
                }
            });
        }
        for(auto& worker:workers) {worker.join();}
        require(!fault.load());equal(copied,4000,4000);
        std::cout<<"macOS callback ledger controls transitions=2000 threads=8 native_uint_boundaries=1 invalid_counts_rejected=1 native_callbacks=0 capture=0 passed\n";
        std::cout<<"macOS capture lease objects acquisitions=4000 releases=4000 threads=8 overflow_rejected=1 native_blocks=0 capture=0 passed\n";
        return 0;
    }catch(const std::exception& error) {std::cerr<<error.what()<<'\n';return 1;}
}
