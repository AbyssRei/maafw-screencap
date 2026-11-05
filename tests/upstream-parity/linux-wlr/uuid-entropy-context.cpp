// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Linux/libstdc++ acceptance only. The linker observes the real default
// MaaUtils function without changing its source or Boost declarations.
#include "MaaUtils/Uuid.h"
#include <filesystem>
#include <fstream>
#include <iostream>
#include <initializer_list>
#include <iterator>
#include <random>
#include <stdexcept>
#include <string>
#include <string_view>
#include <type_traits>

#ifndef __GLIBCXX__
#error "The separate default-entropy observer requires the recorded libstdc++ ABI"
#endif

namespace {
static_assert(sizeof(unsigned)==4);
static_assert(std::is_same_v<std::random_device::result_type,unsigned>);
struct ControlledFailure {};
unsigned calls=0,fail_at=0,seed_word=0;
bool controlled=false;
size_t fds() {
    return static_cast<size_t>(std::distance(std::filesystem::directory_iterator("/proc/self/fd"),
        std::filesystem::directory_iterator{}));
}
}
extern "C" unsigned __real__ZNSt13random_device9_M_getvalEv(void*);
extern "C" unsigned __wrap__ZNSt13random_device9_M_getvalEv(void* self) {
    // Linux x64 Itanium ABI: self borrows the live std::random_device from
    // the original seeder. Forward its address unchanged; no owner escapes.
    ++calls;
    if(calls==fail_at) {throw ControlledFailure{};}
    if(controlled) {return 0x03020100u+seed_word++*0x04040404u;}
    return __real__ZNSt13random_device9_M_getvalEv(self);
}
int main(int argc,char** argv) {
    try {
        if(argc!=3 || std::filesystem::exists(argv[2])) {throw std::runtime_error("usage: uuid-entropy <real|controlled|retry> <new file>");}
        const std::string_view mode=argv[1];
        if(mode!="real" && mode!="controlled" && mode!="retry") {throw std::runtime_error("unknown entropy mode");}
        controlled=mode!="real";
        std::ofstream out(argv[2],std::ios::binary);
        if(!out) {throw std::runtime_error("cannot create entropy records");}
        std::ofstream failure_records;
        if(mode=="retry") {
            const auto path=std::string(argv[2])+".failures";
            if(std::filesystem::exists(path)) {throw std::runtime_error("failure records already exist");}
            failure_records.open(path,std::ios::binary);
            if(!failure_records) {throw std::runtime_error("cannot create failure records");}
        }
        const auto baseline=fds();
        unsigned failures=0;
        if(mode=="retry") {
            for(unsigned word:{1u,5u,12u}) {
                seed_word=0;fail_at=calls+word;
                try {(void)MaaNS::make_uuid();throw std::runtime_error("expected original seed failure");}
                catch(const ControlledFailure&) {++failures;}
                const auto after=fds();
                failure_records<<failures<<'\t'<<word<<'\t'<<calls<<'\t'<<baseline<<'\t'<<after<<'\n';
                if(calls!=fail_at || after!=baseline) {throw std::runtime_error("failed original seed leaked state/resources");}
            }
        }
        seed_word=0;fail_at=0;
        const auto before_success=calls;
        for(unsigned draw=0;draw<256;++draw) {
            out<<draw<<"\tmaafw-wl-buffer-"<<MaaNS::make_uuid()<<'\t'<<calls<<'\n';
            if(calls!=before_success+12 || fds()!=baseline) {throw std::runtime_error("original entropy was not initialized once");}
        }
        if(seed_word!=(controlled?12u:0u)) {throw std::runtime_error("controlled seed consumption differs");}
        if(failure_records.is_open()) {
            failure_records.close();if(!failure_records) {throw std::runtime_error("cannot save seed failures");}
        }
        out.close();if(!out) {throw std::runtime_error("cannot save entropy records");}
        std::cout<<"original MaaUtils entropy mode="<<mode<<" draws=256 successful_seed_words=12 failures="<<failures
                 <<" calls="<<calls<<" warm_calls=0 resources_balanced=1 capture=0 passed\n";
        return 0;
    }catch(const std::exception& error) {std::cerr<<error.what()<<'\n';return 1;}
}
