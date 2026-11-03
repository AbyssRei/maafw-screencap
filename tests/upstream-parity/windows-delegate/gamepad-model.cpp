// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Whole unchanged F Gamepad class/API, with explicit host/DLL boundaries.
#include "unit-fixture.hpp"
#include "gamepad-host-fixture.hpp"
#include "Manager/GamepadControlUnitMgr.h"
#include "MaaControlUnit/GamepadControlUnitAPI.h"
#include <array>
#include <iostream>
#include <set>
namespace MfsGamepadFixture {
unsigned mode=0,next_input=0;
std::set<unsigned> inputs;
void input_live(unsigned id){if(!inputs.insert(id).second){throw std::logic_error("duplicate host owner");}}
void input_drop(unsigned id){if(inputs.erase(id)!=1){std::terminate();}}
}
namespace {
using Source=MAA_CTRL_UNIT_NS::GamepadControlUnitMgr;
template<class Tag,typename Tag::type Value>struct Access {friend typename Tag::type member(Tag){return Value;}};
struct View {using type=std::shared_ptr<MAA_CTRL_UNIT_NS::ControlUnitAPI> Source::*;friend type member(View);};
template struct Access<View,&Source::win32_unit_>;
}
int main(int argc,char** argv) {
    using namespace MfsDelegateFixture;
    try {
        if(argc!=10){throw std::logic_error("expected hwnd type mask initial_host reconnect_host initial_flags reconnect_flags outcome inactive");}
        const auto hwnd=std::stoull(argv[1]),type=std::stoull(argv[2]),method=std::stoull(argv[3]);
        const std::array host_modes{std::stoul(argv[4]),std::stoul(argv[5])};
        const std::array flags{std::stoul(argv[6]),std::stoul(argv[7])};
        const auto outcome=std::stoul(argv[8]),inactive=std::stoul(argv[9]);
        if(host_modes[0]>4 || host_modes[1]>4 || flags[0]>255 || flags[1]>255 || outcome>5 || inactive>2){throw std::logic_error("host model argument range");}
        if(std::string_view(MaaGamepadControlUnitGetVersion())!="mfs-p1-gamepad-host"){throw std::logic_error("original Gamepad API version differs");}
        MaaGamepadControlUnitDestroy(nullptr);
        std::vector<cv::Mat> held;cv::Mat output(2,3,CV_8UC3,cv::Scalar(99,99,99));
        {
            std::unique_ptr<MAA_CTRL_UNIT_NS::GamepadControlUnitAPI,decltype(&MaaGamepadControlUnitDestroy)> unit(
                MaaGamepadControlUnitCreate(reinterpret_cast<void*>(static_cast<uintptr_t>(hwnd)),type,method),MaaGamepadControlUnitDestroy);
            auto* source=dynamic_cast<Source*>(unit.get());if(!source){throw std::logic_error("original Gamepad concrete class missing");}
            std::cout<<"initial_connected="<<unit->connected()<<"\ninitial_capture="<<unit->screencap(output)<<"\ninitial_inactive="<<unit->inactive()<<'\n';
            for(unsigned phase=0;phase<2;++phase) {
                MfsGamepadFixture::mode=static_cast<unsigned>(host_modes[phase]);
                plan={static_cast<uint32_t>(flags[phase]),static_cast<uint32_t>(outcome),static_cast<uint32_t>(inactive)};
                int connected=2;try{connected=unit->connect();}catch(const std::runtime_error&){}
                std::cout<<"connect"<<phase<<'='<<connected<<"\nconnected"<<phase<<'='<<unit->connected()<<'\n';
                const bool view=(source->*member(View{})).use_count()!=0;
                const bool live=view && module_loaded && std::any_of(identities.begin(),identities.end(),[](const auto& item){return item.second.alive;});
                int captured=4,deactivated=4;
                if(!view || live) {
                    try{captured=unit->screencap(output);}catch(const std::runtime_error&){captured=2;}
                    try{deactivated=unit->inactive();}catch(const std::runtime_error&){deactivated=2;}
                }
                // Undefined original stale view is never virtually invoked.
                std::cout<<"capture"<<phase<<'='<<captured<<"\noutput"<<phase<<'='<<bytes(output)<<"\ninactive"<<phase<<'='<<deactivated<<'\n';
                held.push_back(output);
            }
        }
        for(size_t i=0;i<held.size();++i){std::cout<<"held"<<i<<'='<<bytes(held[i])<<'\n';}
        if(module_loaded || !MfsGamepadFixture::inputs.empty() || std::any_of(identities.begin(),identities.end(),[](const auto& item){return item.second.alive;})){throw std::logic_error("outer owners survived destruction");}
        std::cout<<"events=";for(size_t i=0;i<events.size();++i){if(i){std::cout<<',';}std::cout<<events[i];}
        std::cout<<"\nowners_closed=1\n";return 0;
    }catch(const std::exception& error){std::cerr<<"Gamepad host model: "<<error.what()<<'\n';return 1;}
}
