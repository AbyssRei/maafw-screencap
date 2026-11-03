// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "bridge.h"
#include "gamepad-host-fixture.hpp"
#include "MaaUtils/Runtime.h"
#include <windows.h>
#include <array>
#include <fstream>
#include <iostream>
#include <set>
#include <stdexcept>
#include <vector>
namespace MfsGamepadNative {
struct Event { std::string name; unsigned mode; uint64_t created,destroyed,captures; bool loaded; };
inline std::vector<Event> events;
inline void record(const std::string& name) {
    MfsDnEvent native[4096]{};size_t count=0;uint64_t owners=0;
    if(mfs_dn_events(native,std::size(native),&count,&owners)) {throw std::logic_error("host snapshot native event overflow");}
    uint64_t created=0,destroyed=0,captures=0;
    for(size_t i=0;i<count;++i) {
        created+=native[i].kind==1;destroyed+=native[i].kind==5;captures+=native[i].kind==3;
    }
    events.push_back({name,MfsGamepadFixture::mode,created,destroyed,captures,
        GetModuleHandleW((MaaNS::directory/L"MaaWin32ControlUnit.dll").c_str())!=nullptr});
}
struct Trace {
    std::filesystem::path output;
    ~Trace() {
        try {
            std::ofstream stream(output/L"source-gamepad-host.tsv");
            for(const auto& event:events) {stream<<event.name<<'\t'<<event.mode<<'\t'<<event.created<<'\t'<<event.destroyed<<'\t'<<event.captures<<'\t'<<event.loaded<<'\n';}
            stream.close();if(!stream){std::cerr<<"source Gamepad host trace write failed"<<std::endl;}
        } catch(...) {std::cerr<<"source Gamepad host trace export failed"<<std::endl;}
    }
};
}
namespace MfsDelegateFixture {void event(const std::string& name){MfsGamepadNative::record(name);}}
namespace MfsGamepadFixture {
unsigned mode=0,next_input=0;
std::set<unsigned> inputs;
void input_live(unsigned id){if(!inputs.insert(id).second){throw std::logic_error("duplicate host owner");}}
void input_drop(unsigned id){if(inputs.erase(id)!=1){std::terminate();}}
}
