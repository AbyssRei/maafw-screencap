// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "fixture.hpp"
#include <opencv2/core.hpp>
#include <algorithm>
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
