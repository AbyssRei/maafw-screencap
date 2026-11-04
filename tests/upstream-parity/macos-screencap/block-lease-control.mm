// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Actual native Blocks ABI only: no ScreenCaptureKit or desktop capture.
#include "callback-ledger.hpp"
#include "input-weak-observer.hpp"
#include <Block.h>
#import <Foundation/Foundation.h>
#include <atomic>
#include <iostream>
#include <initializer_list>
#include <stdexcept>
#include <thread>
// A source-shaped MRC ownership control. This runs neither the full F class
// nor SCScreenshotManager. Observe dealloc rather than inferred retain counts.
static std::atomic<unsigned> mfs_completion_deallocated=0;
@interface MfsCompletionOwnerProbe : NSObject
@end
@implementation MfsCompletionOwnerProbe
- (void)dealloc {
    mfs_completion_deallocated.fetch_add(1,std::memory_order_relaxed);
    [super dealloc];
}
@end
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
void completion_inputs() {
    for(const bool invoke:{true,false}) {
        const auto before=mfs_completion_deallocated.load();
        auto* filter=[MfsCompletionOwnerProbe new];
        auto* config=[MfsCompletionOwnerProbe new];
        require(filter!=nil && config!=nil);
        // The fixed window and display handlers both contain these create
        // releases. Block_copy separately retains the captured objects.
        Block local=^{[filter release];[config release];};
        Block heap=Block_copy(local);require(heap!=nullptr && heap!=local);
        Block alias=Block_copy(heap);require(alias==heap);Block_release(heap);
        if(invoke) {alias();}
        require(mfs_completion_deallocated.load()==before);
        Block_release(alias);
        require(mfs_completion_deallocated.load()==before+(invoke?2:0));
        if(!invoke) {
            // Fixture-only reclaim AFTER the uninvoked block's final release.
            [filter release];[config release];
            require(mfs_completion_deallocated.load()==before+2);
        }
    }
    require(mfs_completion_deallocated.load()==4);
    std::cout<<"macOS native MRC completion_inputs cases=2 invoked=1 uninvoked=1 heap_alias=2 deallocated=4 fixture_reclaimed_refs=2 capture=0 full_P1_acceptance=0 passed\n";
}
void input_weak_control() {
    MfsMacObserve::InputWeakRegistry registry;
    const auto trace=[](const char* stage,uint64_t request,const MfsMacObserve::InputWeakSnapshot& row,int64_t count) {
        std::cout<<"macOS native input weak stage="<<stage<<" request="<<request
            <<" row_request="<<row.request<<" registered_filter="<<row.registered_filter
            <<" registered_config="<<row.registered_config<<" filter="<<row.filter
            <<" config="<<row.config<<" valid="<<row.valid<<" begin_ns="<<row.begin_ns
            <<" end_ns="<<row.end_ns<<" thread_error="<<row.thread_error
            <<" registry_count="<<count<<" deallocated="<<mfs_completion_deallocated.load()<<std::endl;
    };
    for(uint64_t request=1;request<=2;++request) {
        auto* filter=[MfsCompletionOwnerProbe new];auto* config=[MfsCompletionOwnerProbe new];
        const auto first=registry.bind(request,filter,config);
        trace("bound",request,first,-1); // This boundary has no registry-count query.
        require(first.valid && first.filter==first.registered_filter && first.config==first.registered_config
            && first.filter!=0 && first.config!=0 && first.begin_ns<=first.end_ns && first.thread_error==0);
        auto* filter_alias=[filter retain];auto* config_alias=[config retain];
        std::thread worker([filter,config] {[filter release];[config release];});worker.join();
        auto held=registry.snapshot();
        trace("originals_released",request,held.rows[request-1],held.count);
        require(held.valid && held.count==request);
        require(held.rows[request-1].filter==first.filter && held.rows[request-1].config==first.config);
        [filter_alias release];[config_alias release];
        auto released=registry.snapshot();
        trace("aliases_released",request,released.rows[request-1],released.count);
        require(released.valid && released.count==request);
        for(unsigned i=0;i<released.count;++i) {require(!released.rows[i].filter && !released.rows[i].config);}
    }
    std::cout<<"macOS native input weak control pairs=2 objects=4 joined_worker_releases=4 alias_keeps_live=2 zero_after_final_release=4 native_capture=0 passed\n";
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
        completion_inputs();
        input_weak_control();
        std::cout<<"macOS native block leases cases=4 stack_to_heap=4 heap_alias=1 handler_return_keeps_lease=1 uninvoked_disposal=1 joined_worker_release=1 acquired=12 released=12 live=0 native_callbacks=0 capture=0 passed\n";
        return 0;
    }catch(const std::exception& error) {std::cerr<<error.what()<<'\n';return 1;}
}
