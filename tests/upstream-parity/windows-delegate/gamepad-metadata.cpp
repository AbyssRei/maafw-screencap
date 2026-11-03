// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Separate query-only characterization of the complete unchanged F class/API.
#include "unit-fixture.hpp"
#include "gamepad-host-fixture.hpp"
#include "Manager/GamepadControlUnitMgr.h"
#include "MaaControlUnit/GamepadControlUnitAPI.h"
#include <array>
#include <iostream>
#include <limits>
#include <set>
namespace MfsGamepadFixture {
unsigned mode=0,next_input=0;
std::set<unsigned> inputs;
void input_live(unsigned id){if(!inputs.insert(id).second){throw std::logic_error("duplicate host owner");}}
void input_drop(unsigned id){if(inputs.erase(id)!=1){std::terminate();}}
}
int main() {
    using namespace MfsDelegateFixture;
    try {
        constexpr auto maximum=std::numeric_limits<uint64_t>::max();
        constexpr std::array<uint64_t,4> windows{0,1,0x123456789abcdef0ULL,maximum};
        constexpr std::array<uint64_t,6> types{0,1,2,(1ULL<<63)-1,1ULL<<63,maximum};
        constexpr std::array<uint64_t,6> masks{0,1,63,64,1ULL<<63,maximum};
        size_t count=0;
        for(const auto hwnd:windows){for(const auto type:types){for(const auto method:masks){
            std::array<std::string,2> retained;
            {
                std::unique_ptr<MAA_CTRL_UNIT_NS::GamepadControlUnitAPI,decltype(&MaaGamepadControlUnitDestroy)> unit(
                    MaaGamepadControlUnitCreate(reinterpret_cast<void*>(static_cast<uintptr_t>(hwnd)),type,method),MaaGamepadControlUnitDestroy);
                if(!unit || unit->connected()){throw std::logic_error("unexpected initial connection");}
                for(unsigned phase=0;phase<2;++phase){
                    std::string uuid(phase==0?1:257,'x');
                    if(!unit->request_uuid(uuid)){throw std::logic_error("UUID query returned false");}
                    retained[phase]=uuid;
                    std::cout<<"{\"hwnd\":"<<hwnd<<",\"gamepad_type\":"<<type<<",\"method\":"<<method
                             <<",\"phase\":"<<phase<<",\"uuid\":\""<<uuid<<"\",\"features\":"<<unit->get_features()
                             <<",\"info\":"<<json::value(unit->get_info()).to_string()<<"}\n";
                }
                if(unit->connected()){throw std::logic_error("query changed connection state");}
            }
            if(retained[0]!=retained[1] || retained[0].empty()){throw std::logic_error("returned UUID ownership differs");}
            if(module_loaded || next_id || !identities.empty() || !events.empty() || MfsGamepadFixture::next_input || !MfsGamepadFixture::inputs.empty()){
                throw std::logic_error("query created a capture/input resource");
            }
            ++count;
        }}}
        if(count!=144){throw std::logic_error("query census differs");}
        std::cerr<<"original_cases=144 phases=288 queries=864 input_owners=0 capture_units=0 native_capture=0\n";
        return 0;
    }catch(const std::exception& error){std::cerr<<"Gamepad original metadata: "<<error.what()<<'\n';return 1;}
}
