// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Exact guard ownership control; no Apple API, native sample or capture.
#include "live-sample-hook.hpp"
#include <atomic>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <thread>
namespace {
struct State {std::atomic<unsigned> calls=0,releases=0;};
int32_t visit(void* context,const void* sample,uint32_t call) noexcept {
    auto& state=*static_cast<State*>(context);
    if(sample!=context || call!=2) {return 1;}
    ++state.calls;return 0;
}
void release(void* context) noexcept {++static_cast<State*>(context)->releases;}
void require(bool value) {if(!value) {throw std::runtime_error("live sample hook ownership differs");}}
}
int main() {
    try {
        State first;
        {MfsMacSample::LiveHook guard(&first,visit,release);}
        require(first.calls==0 && first.releases==1);
        State second;
        std::shared_ptr<MfsMacSample::LiveHook> owner;
        {
            MfsMacSample::LiveHook guard(&second,visit,release);
            owner=std::make_shared<MfsMacSample::LiveHook>(std::move(guard));
        }
        require(second.releases==0);
        auto callback=owner;
        owner.reset();
        require(second.releases==0);
        std::thread worker([hook=std::move(callback),&second]() mutable {
            if(hook->visit(hook->context,&second,2)!=0) {std::terminate();}
            hook.reset();
        });
        worker.join();
        require(second.calls==1 && second.releases==1);
        State third;
        try {
            MfsMacSample::LiveHook guard(&third,visit,release);
            // Deliberate pre-transfer exception, not an OOM recovery claim.
            throw std::runtime_error("controlled pre-transfer failure");
        }catch(const std::runtime_error&) {}
        require(third.calls==0 && third.releases==1);
        std::cout<<"live_sample_hook cases=3 visits=1 releases=3 joined_callback=1 native_samples=0 passed\n";
        return 0;
    }catch(const std::exception& error) {std::cerr<<error.what()<<'\n';return 1;}
}
