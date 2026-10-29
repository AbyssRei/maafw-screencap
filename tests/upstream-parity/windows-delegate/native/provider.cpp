// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Real original class ABI; GDI-only source manager, no input implementation.
#include "bridge.h"
#include "MaaFramework/MaaPort.h"
// Export only definitions below, allowing actual missing-export DLL controls.
#undef MAA_CONTROL_UNIT_API
#define MAA_CONTROL_UNIT_API
#include "MaaControlUnit/Win32ControlUnitAPI.h"
#include <windows.h>
#ifndef MFS_RUST_PROVIDER
#include "Screencap/GdiScreencap.h"
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
struct Unavailable : ScreencapBase {
    explicit Unavailable(HWND) { throw std::logic_error("outside GDI-only provider"); }
    std::optional<cv::Mat> screencap() override { return std::nullopt; }
};
using FramePoolWithPseudoMinimizeScreencap=Unavailable;
using DesktopDupScreencap=Unavailable;
using DesktopDupWindowScreencap=Unavailable;
using PrintWindowWithPseudoMinimizeScreencap=Unavailable;
using ScreenDCScreencap=Unavailable;
#include "../../native/manager-shim.h"
#include "manager-time.inc"
#include "manager-source.inc"
}
#endif
namespace {
struct Unit final : MAA_CTRL_UNIT_NS::Win32ControlUnitAPI {
#ifdef MFS_RUST_PROVIDER
    void* context=nullptr;
    Unit(HWND hwnd,uint64_t method) {
        if(mfs_dn_host_create(reinterpret_cast<uintptr_t>(hwnd),method,&context) || !context) { throw std::runtime_error("Rust host create"); }
    }
    ~Unit() override {
        if(mfs_dn_host_destroy(context)) { mfs_dn_event(99,0,0,0,0); }
        mfs_dn_event(5,0,0,0,0);
    }
#else
    Source::Win32ControlUnitMgr manager;
    Unit(HWND hwnd,uint64_t method):manager(hwnd,method,0,0) {}
    ~Unit() override { mfs_dn_event(5,0,0,0,0); }
#endif
    bool connect() override {
        mfs_dn_event(2,0,0,0,0);
        if(mfs_dn_mode()==2) { throw std::runtime_error("injected native connect exception"); }
#ifdef MFS_RUST_PROVIDER
        int32_t result=0;if(mfs_dn_host_connect(context,&result)) { throw std::runtime_error("Rust host connect"); }return result!=0;
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
        if(mfs_dn_host_capture(context,&span,&result)) { throw std::runtime_error("Rust host capture"); }
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
        int32_t result=0;if(mfs_dn_host_inactive(context,&result)) { throw std::runtime_error("Rust host inactive"); }return result!=0;
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
#ifndef MFS_NO_CREATE
extern "C" MaaWin32ControlUnitHandle MaaWin32ControlUnitCreate(void* hwnd,MaaWin32ScreencapMethod method,MaaWin32InputMethod mouse,MaaWin32InputMethod keyboard) {
    mfs_dn_event(1,reinterpret_cast<uintptr_t>(hwnd),method,mouse,keyboard);
    if(mouse || keyboard) { throw std::logic_error("input must be None"); }
    if(mfs_dn_mode()==1) { throw std::runtime_error("injected native create exception"); }
    return new Unit(static_cast<HWND>(hwnd),method);
}
#endif
#ifndef MFS_NO_DESTROY
extern "C" void MaaWin32ControlUnitDestroy(MaaWin32ControlUnitHandle unit) { delete unit; }
#endif
