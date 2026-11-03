// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Exact source ownership/control model with explicit scripted DLL/backend
// boundaries. No ViGEm/input, native module loading or desktop capture.
#include "fixture.hpp"
#include <opencv2/core.hpp>
#include <boost/function.hpp>
#include <algorithm>
#include <array>
#include <filesystem>
#include <iostream>
#include <map>
#include <memory>
#include <stdexcept>
#include <vector>

#include "unit-fixture.hpp"

// Runtime path selection and logging are explicit fixture boundaries. The
// actual prepared MaaUtils LibraryHolder implementation is not rewritten.
#include "original-LibraryHolder.h"
namespace MaaNS { inline std::filesystem::path library_dir() { return "owned-fixture"; } }
namespace MfsDelegateSource {
using namespace MaaNS;
using namespace MAA_CTRL_UNIT_NS;
using HWND=void*;
#include "delegate-loader.inc"
struct GamepadControlUnitMgr {
    bool init_win32_unit();bool screencap(cv::Mat&);bool inactive();
    bool selected() const {
#include "delegate-condition.inc"
    }
    HWND hwnd_=nullptr;
    MaaWin32ScreencapMethod screencap_method_=MaaWin32ScreencapMethod_None;
    // Keep the actual capture members' order; input members are excluded.
    std::unique_ptr<Win32ControlUnitLoader> win32_loader_;
    std::shared_ptr<ControlUnitAPI> win32_unit_;
};
#include "delegate-source.inc"
}

int main(int argc,char** argv) {
    using namespace MfsDelegateFixture;
    static_assert(sizeof(uintptr_t)==8 && sizeof(MaaWin32ScreencapMethod)==8);
    try {
        if(argc!=7) { throw std::logic_error("expected hwnd method initial_flags reconnect_flags outcome inactive"); }
        const auto hwnd=std::stoull(argv[1]),method=std::stoull(argv[2]);
        const std::array flags {std::stoul(argv[3]),std::stoul(argv[4])};
        const auto outcome=std::stoul(argv[5]),inactive=std::stoul(argv[6]);
        if(flags[0]>255 || flags[1]>255 || outcome>5 || inactive>2) { throw std::logic_error("fixture script range"); }
        std::vector<cv::Mat> held;
        cv::Mat output(2,3,CV_8UC3,cv::Scalar(99,99,99));
        {
            MfsDelegateSource::GamepadControlUnitMgr manager;
            manager.hwnd_=reinterpret_cast<void*>(static_cast<uintptr_t>(hwnd));manager.screencap_method_=method;
            std::cout<<"initial_capture="<<manager.screencap(output)<<"\ninitial_inactive="<<manager.inactive()<<'\n';
            for(unsigned phase=0;phase<2;++phase) {
                plan={static_cast<uint32_t>(flags[phase]),static_cast<uint32_t>(outcome),static_cast<uint32_t>(inactive)};
                int initialized=3;
                if(manager.selected()) { try { initialized=manager.init_win32_unit(); }catch(const std::runtime_error&) { initialized=2; } }
                // The independent shared control block can remain live after
                // its no-op pointee expires. Observe that block and the real
                // loader owner, without even reading the stale view pointer.
                const bool view=manager.win32_unit_.use_count()!=0;
                const auto owner=manager.win32_loader_?manager.win32_loader_->handle():nullptr;
                const bool alive=view&&owner&&live(owner);
                std::cout<<"init"<<phase<<'='<<initialized<<"\nloader"<<phase<<'='<<bool(manager.win32_loader_)
                         <<"\nview"<<phase<<'='<<view<<"\nlive"<<phase<<'='<<alive<<"\nmodule"<<phase<<'='<<module_loaded<<'\n';
                int captured=4,deactivated=4;
                // Do not invoke an original stale no-op shared view. Only
                // its control block and actual loader owner are observed;
                // virtual dispatch through the old target would be undefined.
                if(!view || alive) {
                    try { captured=manager.screencap(output); }catch(const std::runtime_error&) { captured=2; }
                    try { deactivated=manager.inactive(); }catch(const std::runtime_error&) { deactivated=2; }
                }
                std::cout<<"capture"<<phase<<'='<<captured<<"\noutput"<<phase<<'='<<bytes(output)<<"\ninactive"<<phase<<'='<<deactivated<<'\n';
                held.push_back(output);
            }
        }
        for(size_t i=0;i<held.size();++i) { std::cout<<"held"<<i<<'='<<bytes(held[i])<<'\n'; }
        if(module_loaded || std::any_of(identities.begin(),identities.end(),[](const auto& item) { return item.second.alive; })) { throw std::logic_error("source model owners remain live"); }
        std::cout<<"events=";
        for(size_t i=0;i<events.size();++i) { if(i) { std::cout<<','; }std::cout<<events[i]; }
        std::cout<<"\nowners_closed=1\n";
        return 0;
    }catch(const std::exception& error) { std::cerr<<"delegate source model: "<<error.what()<<'\n';return 1; }
}
