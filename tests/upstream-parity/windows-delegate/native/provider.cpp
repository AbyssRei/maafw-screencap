// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Real original class ABI; original screenshot factories, no input implementation.
#include "bridge.h"
#include "MaaFramework/MaaPort.h"
// Export only definitions below, allowing actual missing-export DLL controls.
#undef MAA_CONTROL_UNIT_API
#define MAA_CONTROL_UNIT_API
#include "MaaControlUnit/Win32ControlUnitAPI.h"
#include <windows.h>
#ifndef MFS_RUST_PROVIDER
#include "Screencap/GdiScreencap.h"
#include "Screencap/FramePoolWithPseudoMinimizeScreencap.h"
#include "Screencap/DesktopDupScreencap.h"
#include "Screencap/DesktopDupWindowScreencap.h"
#include "Screencap/PrintWindowWithPseudoMinimizeScreencap.h"
#include "Screencap/ScreenDCScreencap.h"
#endif
#include "MaaUtils/Logger.h"
#include <unordered_map>
#include <stdexcept>
#include <opencv2/core.hpp>
#ifndef MFS_RUST_PROVIDER
namespace Source {
using MAA_CTRL_UNIT_NS::ScreencapBase;
struct GdiScreencap : MAA_CTRL_UNIT_NS::GdiScreencap {
    using MAA_CTRL_UNIT_NS::GdiScreencap::GdiScreencap;
    std::optional<cv::Mat> screencap() override { mfs_dn_event(6,0,0,0,0);return MAA_CTRL_UNIT_NS::GdiScreencap::screencap(); }
};
using MAA_CTRL_UNIT_NS::FramePoolWithPseudoMinimizeScreencap;
using MAA_CTRL_UNIT_NS::DesktopDupScreencap;
using MAA_CTRL_UNIT_NS::DesktopDupWindowScreencap;
using MAA_CTRL_UNIT_NS::PrintWindowWithPseudoMinimizeScreencap;
using MAA_CTRL_UNIT_NS::ScreenDCScreencap;
#include "../../native/manager-shim.h"
#include "manager-time.inc"
#include "manager-source.inc"
}
#endif
namespace {
void require_host(int32_t code,const char* operation) {
    if(code==3) { throw std::bad_alloc(); }
    if(code) { throw std::runtime_error(operation); }
}
struct Construction {
    Construction(HWND hwnd,uint64_t method,uint64_t mouse,uint64_t keyboard) {
        mfs_dn_event(1,reinterpret_cast<uintptr_t>(hwnd),method,mouse,keyboard);
        if(mouse || keyboard) { throw std::logic_error("input must be None"); }
        if(mfs_dn_mode()==1) { throw std::runtime_error("injected native create exception"); }
    }
};
struct Unit final : MAA_CTRL_UNIT_NS::Win32ControlUnitAPI {
    Construction construction;
#ifdef MFS_RUST_PROVIDER
    void* context=nullptr;
    Unit(HWND hwnd,uint64_t method,uint64_t mouse,uint64_t keyboard):construction(hwnd,method,mouse,keyboard) {
        require_host(mfs_dn_host_create(reinterpret_cast<uintptr_t>(hwnd),method,&context),"Rust host create");
        if(!context) { throw std::runtime_error("Rust host create returned no owner"); }
    }
    ~Unit() override {
        if(mfs_dn_host_destroy(context)) { mfs_dn_event(99,0,0,0,0); }
        mfs_dn_event(5,0,0,0,0);
    }
#else
    Source::Win32ControlUnitMgr manager;
    Unit(HWND hwnd,uint64_t method,uint64_t mouse,uint64_t keyboard):construction(hwnd,method,mouse,keyboard),manager(hwnd,method,mouse,keyboard) {}
    ~Unit() override { mfs_dn_event(5,0,0,0,0); }
#endif
    bool connect() override {
        mfs_dn_event(2,0,0,0,0);
        if(mfs_dn_mode()==2) { throw std::runtime_error("injected native connect exception"); }
#ifdef MFS_RUST_PROVIDER
        int32_t result=0;require_host(mfs_dn_host_connect(context,&result),"Rust host connect");return result!=0;
#else
        return manager.connect();
#endif
    }
    bool screencap(cv::Mat& image) override {
        mfs_dn_event(3,0,0,0,0);
        if(mfs_dn_mode()==3 || mfs_dn_mode()==4) {
            image=cv::Mat(2,3,CV_8UC3,cv::Scalar(7,9,11));
            if(mfs_dn_mode()==4) { throw std::runtime_error("injected native post-write exception"); }
            return false;
        }
#ifdef MFS_RUST_PROVIDER
        MfsDnImage span={};int32_t result=0;
        require_host(mfs_dn_host_capture(context,&span,&result),"Rust host capture");
        if(result) {
            if(!span.data) { image.release(); }
            else {
                if(span.width<=0 || span.height<=0 || span.stride!=static_cast<size_t>(span.width)*3 || span.length/span.stride!=static_cast<size_t>(span.height) || span.length%span.stride) { throw std::runtime_error("Rust host span"); }
                // Explicit first boundary copy: borrowed Rust bytes -> provider
                // Mat. Bridge capture performs the second independent copy.
                image=cv::Mat(span.height,span.width,CV_8UC3,const_cast<uint8_t*>(span.data),span.stride).clone();
            }
        }
        return result!=0;
#else
        return manager.screencap(image);
#endif
    }
    bool inactive() override {
        mfs_dn_event(4,0,0,0,0);
#ifdef MFS_RUST_PROVIDER
        int32_t result=0;require_host(mfs_dn_host_inactive(context,&result),"Rust host inactive");return result!=0;
#else
        return manager.inactive();
#endif
    }
    bool connected() const override { throw std::logic_error("excluded connected query"); }
    bool request_uuid(std::string&) override { throw std::logic_error("excluded UUID query"); }
    MaaControllerFeature get_features() const override { throw std::logic_error("excluded features"); }
    json::object get_info() const override { throw std::logic_error("excluded metadata"); }
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
}
// The original export bodies construct this facade through the original
// concrete class name. Interface inheritance and exported declarations remain
// the actual original ABI; input operations remain deliberately excluded.
namespace MAA_CTRL_UNIT_NS { using Win32ControlUnitMgr=::Unit; }
#ifndef MFS_NO_CREATE
#include "delegate-factory-create.inc"
#endif
#ifndef MFS_NO_DESTROY
#include "delegate-factory-destroy.inc"
#endif
