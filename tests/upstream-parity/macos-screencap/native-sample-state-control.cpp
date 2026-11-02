// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "native-sample-state-control.hpp"
#include <array>
#include <functional>
#include <iostream>
#include <latch>
#include <thread>

int main() {
    using namespace MfsSampleControl;
    std::atomic<unsigned> faults{0};
    const auto check=[&](bool value) {if(!value) {++faults;}};
    std::array<std::function<void()>,2> content,samples;
    std::array<std::weak_ptr<State>,2> weak;
    auto first=std::make_shared<State>(1);weak[0]=first;
    std::latch second_queued(1);
    const auto bind=[&](size_t index) {
        // Same ThreadScope template as the native content/sample forwarding.
        // Captured owners remain independent after both caller scopes end.
        return [&,index,owner=Scope::current()] {
            Scope forwarded(owner);check(other_translation_unit_id()==index+1);
            samples[index]=[&,index,owner=Scope::current()] {
                {
                    Scope forwarded(owner);check(other_translation_unit_id()==index+1);
                    {Scope nested(std::make_shared<State>(99));check(other_translation_unit_id()==99);}
                    check(other_translation_unit_id()==index+1);
                }
                check(other_translation_unit_id()==0);
            };
        };
    };
    {Scope caller(first);check(other_translation_unit_id()==1);content[0]=bind(0);}
    first.reset();check(!weak[0].expired());check(other_translation_unit_id()==0);
    {
        auto second=std::make_shared<State>(2);weak[1]=second;
        Scope caller(second);content[1]=bind(1);
        std::thread newer([&] {content[1]();check(other_translation_unit_id()==0);second_queued.count_down();});
        std::thread older([&] {second_queued.wait();content[0]();check(other_translation_unit_id()==0);});
        newer.join();older.join();check(other_translation_unit_id()==2);
    }
    content={};check(other_translation_unit_id()==0);
    check(!weak[0].expired() && !weak[1].expired());
    std::thread native_samples([&] {samples[0]();samples[1]();check(other_translation_unit_id()==0);});
    native_samples.join();samples={};
    check(weak[0].expired() && weak[1].expired() && State::live.load()==0);
    if(faults.load()!=0) {std::cerr<<"native sample association faults="<<faults.load()<<'\n';return 1;}
    std::cout<<"native sample association_control threads=3 late_content=1 reverse_sample=1 cross_translation_unit=1 nested_restore=1 cpu_owners_released=1 capture=0 passed\n";
}
