// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Actual native Blocks ABI only: no ScreenCaptureKit or desktop capture.
#include "callback-ledger.hpp"
#include <Block.h>
#include <atomic>
#include <iostream>
#include <stdexcept>
#include <thread>
namespace {
using MfsMacObserve::BlockLease;
using MfsMacObserve::CallbackLedger;
using Block=void (^)(void);
void require(bool value) {if(!value) {throw std::runtime_error("native block capture lease differs");}}
void equal(const CallbackLedger& ledger,uint32_t acquired,uint32_t released) {
    const auto value=ledger.snapshot();
    require(value.valid && value.submitted==acquired && value.forwarded==released
        && value.pending==acquired-released);
}
Block escape(CallbackLedger& ledger,std::atomic<unsigned>& calls) {
    const BlockLease lease(ledger);
    auto* counter=&calls;
    Block local=^{lease.touch();counter->fetch_add(1,std::memory_order_relaxed);};
    equal(ledger,2,0); // Local token plus its stack-block capture.
    Block heap=Block_copy(local);
    require(heap!=nullptr && heap!=local);equal(ledger,3,0);
    return heap; // Both lexical captures are destroyed before the caller resumes.
}
}
int main() {
    try {
        CallbackLedger aliased;std::atomic<unsigned> alias_calls=0;
        {
            const BlockLease lease(aliased);
            auto* counter=&alias_calls;
            Block local=^{lease.touch();counter->fetch_add(1,std::memory_order_relaxed);};
            equal(aliased,2,0);
            Block heap=Block_copy(local);require(heap!=nullptr && heap!=local);equal(aliased,3,0);
            Block second=Block_copy(heap);require(second==heap);equal(aliased,3,0);
            heap();second();require(alias_calls==2);equal(aliased,3,0);
            Block_release(heap);equal(aliased,3,0);
            Block_release(second);equal(aliased,3,1);
        }
        equal(aliased,3,3);
        CallbackLedger returned;std::atomic<unsigned> calls=0;
        Block heap=escape(returned,calls);equal(returned,3,2);
        heap();require(calls==1);equal(returned,3,2); // Handler return is not disposal.
        Block_release(heap);equal(returned,3,3);
        CallbackLedger uninvoked;std::atomic<unsigned> absent=0;
        Block cancelled=escape(uninvoked,absent);equal(uninvoked,3,2);
        Block_release(cancelled);equal(uninvoked,3,3);require(absent==0);
        CallbackLedger threaded;std::atomic<unsigned> worker_calls=0;
        Block worker_block=escape(threaded,worker_calls);equal(threaded,3,2);
        std::thread worker([worker_block] {worker_block();Block_release(worker_block);});
        worker.join();equal(threaded,3,3);require(worker_calls==1);
        std::cout<<"macOS native block leases cases=4 stack_to_heap=4 heap_alias=1 handler_return_keeps_lease=1 uninvoked_disposal=1 joined_worker_release=1 acquired=12 released=12 live=0 native_callbacks=0 capture=0 passed\n";
        return 0;
    }catch(const std::exception& error) {std::cerr<<error.what()<<'\n';return 1;}
}
