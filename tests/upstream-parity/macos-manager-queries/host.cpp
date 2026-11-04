// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Exact manager bodies with explicit availability/capture/input-owner models.
// This translation unit never constructs native input or ScreenCaptureKit.
#include "host.h"
#include "Base/UnitBase.h"
#include "MaaFramework/MaaDef.h"
#include <meojson/json.hpp>
#include <cstring>
#include <limits>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>

namespace MfsMacHost {
struct Active {const MfsMhScript* script;MfsMhReport* report;uint32_t phase=0;};
thread_local Active* active=nullptr;
void event(uint32_t kind,uint32_t unit,uint32_t call=0) noexcept {
    auto& r=*active->report;
    if(r.event_count>=32) {r.overflow=1;return;}
    r.events[r.event_count++]={kind,unit,active->phase,call};
}
using ScreencapBase=MfsOriginalMac::ScreencapBase;
struct ScreenCaptureKitScreencap : ScreencapBase {
    uint32_t id;
    explicit ScreenCaptureKitScreencap(uint32_t window) : id(++active->report->capture_creates) {
        event(1,id);
        if(window!=active->script->window_id) {throw std::logic_error("capture target changed");}
        if(active->script->fail_capture[active->phase]) {throw std::runtime_error("controlled capture constructor failure");}
    }
    ~ScreenCaptureKitScreencap() override {event(3,id);}
    std::optional<cv::Mat> screencap() override {
        const auto call=active->report->calls++;event(2,id,call);
        if(call>=3) {throw std::logic_error("too many capture calls");}
        switch(active->script->outcomes[call]) {
        case 0:return std::nullopt;
        case 1:return cv::Mat(1,1,CV_8UC1,cv::Scalar(20+call));
        case 2:return cv::Mat{};
        case 3:throw std::runtime_error("controlled capture failure");
        default:throw std::logic_error("unknown capture outcome");
        }
    }
};
struct InputBase {
    uint32_t id;
    InputBase(uint32_t window,uint64_t method) : id(++active->report->input_creates) {
        event(4,id);
        if(window!=active->script->window_id || method!=active->script->input_method) {throw std::logic_error("input target/selector changed");}
        if(active->script->fail_input[active->phase]) {throw std::runtime_error("controlled host input constructor failure");}
    }
    virtual ~InputBase() {event(6,id);}
    virtual MaaControllerFeature get_features() const {
        event(5,id,active->report->queries++);
        return active->script->features[active->phase];
    }
};
struct GlobalEventInput : InputBase {
    explicit GlobalEventInput(uint32_t window) : InputBase(window,MaaMacOSInputMethod_GlobalEvent) {}
};
struct PostToPidInput : InputBase {
    explicit PostToPidInput(uint32_t window) : InputBase(window,MaaMacOSInputMethod_PostToPid) {}
};
#define MFS_MAC_HOST_INPUT_TYPES
#define MFS_MAC_QUERY_CONSUMER
#include "manager-shim.h"
#undef MFS_MAC_QUERY_CONSUMER
#undef MFS_MAC_HOST_INPUT_TYPES
#define LogWarn mfs_mac_null_log
#define LogError mfs_mac_null_log
#define __builtin_available(...) (active->script->available[active->phase]!=0)
#include "macos-manager-source.inc"
#undef __builtin_available
#undef LogError
#undef LogWarn
#include "macos-info-source.inc"
}

int32_t mfs_mh_run(const MfsMhScript* script,MfsMhReport* report) noexcept {
    if(!report) {return 1;}*report={};
    if(!script || MfsMacHost::active) {return 1;}
    for(auto value:script->available) {if(value>1) {return 1;}}
    for(auto value:script->fail_capture) {if(value>1) {return 1;}}
    for(auto value:script->fail_input) {if(value>1) {return 1;}}
    for(auto value:script->outcomes) {if(value>3) {return 1;}}
    MfsMacHost::Active state{script,report};MfsMacHost::active=&state;
    struct Reset {~Reset(){MfsMacHost::active=nullptr;}} reset;
    try {
        cv::Mat output(1,1,CV_8UC1,cv::Scalar(99));
        {
            MfsMacHost::MacOSControlUnitMgr manager(script->window_id,script->method,script->input_method);
            const auto capture=[&](size_t slot) {
                try {report->captures[slot]=manager.screencap(output);}
                catch(...) {report->captures[slot]=2;}
                report->pixels[slot]=output.empty() ? 0 : output.at<uint8_t>(0,0);
            };
            report->connected[0]=manager.connected();report->features[0]=manager.get_features();capture(0);
            for(uint32_t phase=0;phase<2;++phase) {
                state.phase=phase;
                try {report->connect[phase]=manager.connect();}catch(...) {report->connect[phase]=2;}
                report->connected[phase+1]=manager.connected();
                report->capture_owner[phase]=manager.screencap_ ? static_cast<MfsMacHost::ScreenCaptureKitScreencap*>(manager.screencap_.get())->id : 0;
                report->input_owner[phase]=manager.input_ ? manager.input_->id : 0;
                report->features[phase==0 ? 1 : 3]=manager.get_features();capture(phase==0 ? 1 : 3);
                if(phase==0) {
                    if(!manager.inactive()) {throw std::logic_error("inactive changed");}
                    report->features[2]=manager.get_features();capture(2);
                }
            }
            const auto info=json::value(manager.get_info()).to_string();
            if(info.size()>sizeof(report->info)) {throw std::logic_error("info capacity");}
            report->info_length=static_cast<uint32_t>(info.size());std::memcpy(report->info,info.data(),info.size());
        }
        return report->overflow ? 2 : 0;
    }catch(...) {return 2;}
}
size_t mfs_mh_layout(uint32_t index) noexcept {
    constexpr size_t values[]={1,sizeof(MfsMhScript),alignof(MfsMhScript),
        offsetof(MfsMhScript,method),offsetof(MfsMhScript,input_method),offsetof(MfsMhScript,window_id),
        offsetof(MfsMhScript,available),offsetof(MfsMhScript,fail_capture),offsetof(MfsMhScript,fail_input),offsetof(MfsMhScript,features),offsetof(MfsMhScript,outcomes),
        sizeof(MfsMhReport),alignof(MfsMhReport),offsetof(MfsMhReport,connect),offsetof(MfsMhReport,connected),
        offsetof(MfsMhReport,capture_owner),offsetof(MfsMhReport,input_owner),offsetof(MfsMhReport,features),
        offsetof(MfsMhReport,captures),offsetof(MfsMhReport,pixels),offsetof(MfsMhReport,capture_creates),offsetof(MfsMhReport,input_creates),
        offsetof(MfsMhReport,calls),offsetof(MfsMhReport,queries),offsetof(MfsMhReport,event_count),offsetof(MfsMhReport,overflow),
        offsetof(MfsMhReport,events),offsetof(MfsMhReport,info_length),offsetof(MfsMhReport,info),
        sizeof(MfsMhEvent),alignof(MfsMhEvent),offsetof(MfsMhEvent,kind),offsetof(MfsMhEvent,unit),offsetof(MfsMhEvent,phase),offsetof(MfsMhEvent,call),
        MaaMacOSInputMethod_None,MaaMacOSInputMethod_GlobalEvent,MaaMacOSInputMethod_PostToPid,
        MaaControllerFeature_UseMouseDownAndUpInsteadOfClick,MaaControllerFeature_UseKeyboardDownAndUpInsteadOfClick};
    return index<sizeof(values)/sizeof(values[0]) ? values[index] : std::numeric_limits<size_t>::max();
}
