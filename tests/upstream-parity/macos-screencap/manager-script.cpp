// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "manager-evidence.h"
#include "Base/UnitBase.h"
#include "MaaFramework/MaaDef.h"
#include <cstddef>
#include <cstring>
#include <limits>
#include <memory>
#include <new>
#include <stdexcept>
#include <string>

namespace MfsMacManagerModel {
struct Active { const ::MfsMacManagerScript* script;MfsMacManagerReport* report;uint32_t phase=0; };
thread_local Active* active=nullptr;
void event(uint32_t kind,uint32_t unit,uint32_t call=0) noexcept {
    auto& report=*active->report;
    if(report.event_count>=16) { report.overflow=1;return; }
    report.events[report.event_count++]={kind,unit,call,0};
}
using ScreencapBase=MfsOriginalMac::ScreencapBase;
struct ScreenCaptureKitScreencap : ScreencapBase {
    uint32_t id;
    explicit ScreenCaptureKitScreencap(uint32_t window) : id(++active->report->creates) {
        event(1,id);
        if(window!=active->script->window_id) { throw std::logic_error("window identity changed"); }
        // A test backend constructor failure exercises the exact assignment/
        // unwind boundary, not real allocator exhaustion or native SCK failure.
        if(active->script->fail_create[active->phase]) { throw std::bad_alloc(); }
    }
    ~ScreenCaptureKitScreencap() override { event(3,id); }
    std::optional<cv::Mat> screencap() override {
        const auto call=active->report->calls++;
        event(2,id,call);
        if(call>=3) { throw std::logic_error("unexpected scripted capture count"); }
        switch(active->script->outcomes[call]) {
        case 0:return std::nullopt;
        case 1:return cv::Mat(1,1,CV_8UC1,cv::Scalar(20+call));
        case 2:return cv::Mat{};
        case 3:throw std::runtime_error("controlled backend exception");
        default:throw std::logic_error("invalid outcome");
        }
    }
};
void mfs_mac_input_attempt() { ++active->report->input_attempts; }
bool mfs_mac_available() { return active->script->available[active->phase]!=0; }
#include "manager-shim.h"
#define LogWarn mfs_mac_null_log
#define LogError mfs_mac_null_log
#define __builtin_available(...) mfs_mac_available()
#include "macos-manager-source.inc"
#undef __builtin_available
#undef LogError
#undef LogWarn
}

int32_t mfs_mac_manager_script(const MfsMacManagerScript* script,MfsMacManagerReport* report) noexcept {
    if(!report) { return 1; }
    *report={};
    if(!script || MfsMacManagerModel::active) { return 1; }
    for(auto value:script->available) { if(value>1) { return 1; } }
    for(auto value:script->fail_create) { if(value>1) { return 1; } }
    for(auto value:script->outcomes) { if(value>3) { return 1; } }
    MfsMacManagerModel::Active state{script,report};
    MfsMacManagerModel::active=&state;
    struct Reset { ~Reset() { MfsMacManagerModel::active=nullptr; } } reset;
    try {
        cv::Mat output(1,1,CV_8UC1,cv::Scalar(99));
        {
            MfsMacManagerModel::MacOSControlUnitMgr manager(script->window_id,script->method,MaaMacOSInputMethod_None);
            auto capture=[&](size_t index) {
                try { report->capture_status[index]=manager.screencap(output); }
                catch(...) { report->capture_status[index]=2; }
                report->output_tokens[index]=output.empty() ? 0 : output.at<uint8_t>(0,0);
            };
            report->connected[0]=manager.connected();
            capture(0);
            for(uint32_t phase=0;phase<2;++phase) {
                state.phase=phase;
                try { report->connect_status[phase]=manager.connect(); }
                catch(...) { report->connect_status[phase]=2; }
                report->connected[phase+1]=manager.connected();
                report->selected[phase]=manager.screencap_ ?
                    static_cast<MfsMacManagerModel::ScreenCaptureKitScreencap*>(manager.screencap_.get())->id : 0;
                capture(phase==0 ? 1 : 3);
                if(phase==0) { report->inactive=manager.inactive();capture(2); }
            }
            std::string uuid;
            report->uuid_success=manager.request_uuid(uuid);
            if(uuid.size()>10) { return 2; }
            report->uuid_length=static_cast<uint32_t>(uuid.size());
            std::memcpy(report->uuid,uuid.data(),uuid.size());
        }
        return 0;
    }catch(...) { return 2; }
}
size_t mfs_mac_manager_layout(uint32_t index) noexcept {
    constexpr size_t values[]={1,
        sizeof(MfsMacManagerScript),alignof(MfsMacManagerScript),
        offsetof(MfsMacManagerScript,method),offsetof(MfsMacManagerScript,window_id),
        offsetof(MfsMacManagerScript,available),offsetof(MfsMacManagerScript,fail_create),offsetof(MfsMacManagerScript,outcomes),
        sizeof(MfsMacManagerReport),alignof(MfsMacManagerReport),
        offsetof(MfsMacManagerReport,connect_status),offsetof(MfsMacManagerReport,connected),
        offsetof(MfsMacManagerReport,selected),offsetof(MfsMacManagerReport,capture_status),offsetof(MfsMacManagerReport,output_tokens),
        offsetof(MfsMacManagerReport,inactive),offsetof(MfsMacManagerReport,uuid_success),offsetof(MfsMacManagerReport,uuid_length),
        offsetof(MfsMacManagerReport,uuid),offsetof(MfsMacManagerReport,reserved),offsetof(MfsMacManagerReport,creates),
        offsetof(MfsMacManagerReport,calls),offsetof(MfsMacManagerReport,input_attempts),offsetof(MfsMacManagerReport,overflow),
        offsetof(MfsMacManagerReport,event_count),offsetof(MfsMacManagerReport,events),
        sizeof(MfsMacManagerEvent),alignof(MfsMacManagerEvent),offsetof(MfsMacManagerEvent,kind),
        offsetof(MfsMacManagerEvent,unit),offsetof(MfsMacManagerEvent,call),offsetof(MfsMacManagerEvent,reserved),
        MaaMacOSScreencapMethod_None,MaaMacOSScreencapMethod_ScreenCaptureKit};
    return index<sizeof(values)/sizeof(values[0]) ? values[index] : std::numeric_limits<size_t>::max();
}
