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

namespace MfsDelegateFixture {
Plan plan;
bool module_loaded=false;
std::vector<std::string> events;
struct Identity { unsigned id;bool alive; };
std::map<uintptr_t,Identity> identities;
unsigned next_id=0;
void event(const std::string& value) { events.push_back(value); }
struct Unit final : MAA_CTRL_UNIT_NS::Win32ControlUnitAPI {
    unsigned id=++next_id;
    Unit() { if(id>255) { throw std::logic_error("fixture image token overflow"); }identities[reinterpret_cast<uintptr_t>(static_cast<MAA_CTRL_UNIT_NS::ControlUnitAPI*>(this))]={id,true}; }
    ~Unit() override { event("unit.drop:"+std::to_string(id));identities.at(reinterpret_cast<uintptr_t>(static_cast<MAA_CTRL_UNIT_NS::ControlUnitAPI*>(this))).alive=false; }
    bool connect() override {
        event("unit.connect:"+std::to_string(id));
        if(plan.flags&128) { throw std::runtime_error("scripted connect exception"); }
        return !(plan.flags&16);
    }
    bool screencap(cv::Mat& image) override {
        event("unit.capture:"+std::to_string(id));
        if(plan.outcome==0) { return false; }
        if(plan.outcome==3) { throw std::runtime_error("scripted capture exception"); }
        if(plan.outcome==2) { image.release();return true; }
        image=cv::Mat(2,3,CV_8UC3,cv::Scalar(7,id,11));
        if(plan.outcome==5) { throw std::runtime_error("scripted capture exception after output"); }
        return plan.outcome!=4;
    }
    bool inactive() override {
        event("unit.inactive:"+std::to_string(id));
        if(plan.inactive==2) { throw std::runtime_error("scripted inactive exception"); }
        return plan.inactive==1;
    }
    // Required original virtual ABI only; accidental excluded calls reject.
    bool connected() const override { throw std::logic_error("excluded connected query"); }
    bool request_uuid(std::string&) override { throw std::logic_error("excluded UUID query"); }
    MaaControllerFeature get_features() const override { throw std::logic_error("excluded features query"); }
    json::object get_info() const override { throw std::logic_error("excluded metadata query"); }
#define MFS_EXCLUDED(name,args) bool name args override { throw std::logic_error("excluded input/app operation"); }
    MFS_EXCLUDED(start_app,(const std::string&))
    MFS_EXCLUDED(stop_app,(const std::string&))
    MFS_EXCLUDED(click,(int,int))
    MFS_EXCLUDED(swipe,(int,int,int,int,int))
    MFS_EXCLUDED(touch_down,(int,int,int,int))
    MFS_EXCLUDED(touch_move,(int,int,int,int))
    MFS_EXCLUDED(touch_up,(int))
    MFS_EXCLUDED(click_key,(int))
    MFS_EXCLUDED(input_text,(const std::string&))
    MFS_EXCLUDED(key_down,(int))
    MFS_EXCLUDED(key_up,(int))
    MFS_EXCLUDED(scroll,(int,int))
    MFS_EXCLUDED(relative_move,(int,int))
#undef MFS_EXCLUDED
};
MaaWin32ControlUnitHandle create(void* hwnd,MaaWin32ScreencapMethod method,MaaWin32InputMethod mouse,MaaWin32InputMethod keyboard) {
    event("unit.create:"+std::to_string(reinterpret_cast<uintptr_t>(hwnd))+":"+std::to_string(method)+":"+std::to_string(mouse)+":"+std::to_string(keyboard));
    if(mouse!=MaaWin32InputMethod_None || keyboard!=MaaWin32InputMethod_None) { throw std::logic_error("input selector was not None"); }
    if(plan.flags&64) { throw std::runtime_error("scripted create exception"); }
    return plan.flags&8 ? nullptr : new Unit;
}
void destroy(MaaWin32ControlUnitHandle unit) { delete unit; }
bool live(MAA_CTRL_UNIT_NS::ControlUnitAPI* unit) {
    const auto found=identities.find(reinterpret_cast<uintptr_t>(unit));
    return found!=identities.end() && found->second.alive;
}
std::string bytes(const cv::Mat& image) {
    if(image.empty()) { return "empty"; }
    if(image.rows!=2 || image.cols!=3 || image.type()!=CV_8UC3) { throw std::logic_error("unexpected actual Mat layout"); }
    constexpr char hex[]="0123456789abcdef";
    std::string result;
    for(int y=0;y<image.rows;++y) { for(int x=0;x<image.cols*3;++x) { const auto b=image.ptr<uint8_t>(y)[x];result.push_back(hex[b>>4]);result.push_back(hex[b&15]); } }
    return result;
}
}

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
