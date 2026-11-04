// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "native-host.h"
#include "Screencap/ScreenCaptureKitScreencap.h"
#include "MaaFramework/MaaDef.h"
#include <meojson/json.hpp>
#include <array>
#include <filesystem>
#include <fstream>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace MfsMacNativeHost {
using Event=std::array<uint64_t,4>;
struct State {
    uint32_t window=0,phase=0,creates=0,input_creates=0,calls=0,queries=0,input_drops=0;
    uint64_t input=0;
    std::vector<Event> events;
};
thread_local State* active=nullptr;
void event(uint64_t kind,uint64_t id,uint64_t call=0) {
    active->events.push_back({kind,id,active->phase,call});
}
struct InputFailure : std::runtime_error {InputFailure():std::runtime_error("controlled host input constructor failure") {}};
using ScreencapBase=MfsOriginalMac::ScreencapBase;
struct ScreenCaptureKitScreencap : MfsOriginalMac::ScreenCaptureKitScreencap {
    uint32_t id;
    explicit ScreenCaptureKitScreencap(uint32_t window)
        : MfsOriginalMac::ScreenCaptureKitScreencap(window),id(++active->creates) {
        if(window!=active->window) {throw std::logic_error("native capture target changed");}
        event(1,id);
    }
    ~ScreenCaptureKitScreencap() override {event(3,id);}
    std::optional<cv::Mat> screencap() override {
        event(2,id,active->calls++);
        return MfsOriginalMac::ScreenCaptureKitScreencap::screencap();
    }
};
struct InputBase {
    uint32_t id;
    explicit InputBase(uint32_t window,uint64_t input):id(++active->input_creates) {
        event(4,id);
        if(window!=active->window || input!=active->input) {throw std::logic_error("host input configuration changed");}
        if(active->phase==1) {throw InputFailure();}
    }
    virtual ~InputBase() {event(6,id);++active->input_drops;}
    virtual MaaControllerFeature get_features() const {
        event(5,id,active->queries++);
        constexpr uint64_t features[]={5,std::numeric_limits<uint64_t>::max(),4};
        return features[active->phase];
    }
};
struct GlobalEventInput : InputBase {
    explicit GlobalEventInput(uint32_t window):InputBase(window,MaaMacOSInputMethod_GlobalEvent) {}
};
struct PostToPidInput : InputBase {
    explicit PostToPidInput(uint32_t window):InputBase(window,MaaMacOSInputMethod_PostToPid) {}
};
#define MFS_MAC_HOST_INPUT_TYPES
#define MFS_MAC_QUERY_CONSUMER
#include "manager-shim.h"
#undef MFS_MAC_QUERY_CONSUMER
#undef MFS_MAC_HOST_INPUT_TYPES
#define LogWarn mfs_mac_null_log
#define LogError mfs_mac_null_log
// Native __builtin_available and the original capture implementation remain.
#include "macos-manager-source.inc"
#undef LogError
#undef LogWarn
#include "macos-info-source.inc"

template<class Range> void numbers(std::ostream& out,const Range& values) {
    out<<'[';bool comma=false;
    for(auto value:values) {if(comma) {out<<',';}out<<value;comma=true;}
    out<<']';
}
void events(std::ostream& out,const State& state,bool inputs_only) {
    out<<'[';bool comma=false;
    for(const auto& row:state.events) {
        if(inputs_only && row[0]!=4 && row[0]!=5 && row[0]!=6) {continue;}
        if(comma) {out<<',';}numbers(out,row);comma=true;
    }
    out<<']';
}
void save(const cv::Mat& image,const std::filesystem::path& path) {
    if(image.type()!=CV_8UC3 || image.dims!=2 || image.cols!=47 || image.rows!=31) {
        throw std::runtime_error("native host capture layout differs");
    }
    for(int y=2;y<29;++y) {for(int x=2;x<45;++x) {
        if(image.at<cv::Vec3b>(y,x)!=cv::Vec3b(0,255,0)) {
            throw std::runtime_error("native host known interior pixel differs");
        }
    }}
    std::ofstream out(path,std::ios::binary);
    for(int y=0;y<31;++y) {out.write(reinterpret_cast<const char*>(image.ptr(y)),141);}
    out.close();if(!out) {throw std::runtime_error("native host image write failed");}
}
void run(uint32_t window,const std::filesystem::path& root) {
    if(!std::filesystem::is_directory(root) || std::filesystem::exists(root/"source.tsv")) {
        throw std::runtime_error("native host requires a fresh prepared directory");
    }
    std::ofstream records(root/"source.tsv"),owners(root/"source-events.tsv");
    std::vector<cv::Mat> held;held.reserve(60);
    for(uint32_t index=0;index<20;++index) {
        State state;state.window=window;state.input=1+index%2;state.events.reserve(32);
        active=&state;
        struct Reset {~Reset(){active=nullptr;}} reset;
        std::array<uint64_t,18> values{};
        std::string info;
        {
            MacOSControlUnitMgr manager(window,MaaMacOSScreencapMethod_ScreenCaptureKit,state.input);
            values[0]=manager.connected();values[1]=manager.get_features();
            std::string uuid;
            if(!manager.request_uuid(uuid) || uuid!=std::to_string(window)) {throw std::runtime_error("native host UUID changed");}
            cv::Mat output;
            for(uint32_t phase=0;phase<3;++phase) {
                state.phase=phase;
                uint64_t connected=0;
                try {connected=manager.connect();}catch(const InputFailure&) {connected=2;}
                const auto slot=phase==0 ? 2U : (phase==1 ? 7U : 11U);
                values[slot]=connected;values[slot+1]=manager.connected();values[slot+2]=manager.get_features();
                values[slot+3]=manager.screencap(output);
                if(values[slot+3]!=1) {throw std::runtime_error("native host original capture returned false");}
                save(output,root/("source-current-"+std::to_string(index)+"-"+std::to_string(phase)+".bgr"));
                held.push_back(output);
                if(phase==0) {
                    if(!manager.inactive()) {throw std::runtime_error("native host inactive changed");}
                    values[6]=manager.get_features();
                }
            }
            info=json::value(manager.get_info()).to_string();
        }
        values[15]=state.input_creates;values[16]=state.input_drops;values[17]=state.queries;
        if(state.creates!=3 || state.calls!=3) {throw std::runtime_error("native host source call count changed");}
        records<<index<<'\t'<<state.input<<'\t'<<window<<'\t';numbers(records,values);
        records<<'\t';events(records,state,true);records<<'\t'<<info<<'\n';
        owners<<index<<'\t';events(owners,state,false);owners<<'\n';
    }
    for(uint32_t index=0;index<20;++index) {for(uint32_t call=0;call<3;++call) {
        save(held[index*3+call],root/("source-held-"+std::to_string(index)+"-"+std::to_string(call)+".bgr"));
    }}
    records.close();owners.close();
    if(!records || !owners) {throw std::runtime_error("native host record write failed");}
}
}
int32_t mfs_mh_native_run(uint32_t window,const char* directory) noexcept {
    if(!window || !directory || MfsMacNativeHost::active) {return 1;}
    try {@autoreleasepool {MfsMacNativeHost::run(window,directory);}return 0;}
    catch(const std::exception& error) {
        try {std::ofstream out(std::filesystem::path(directory)/"source-error.txt");out<<error.what()<<'\n';}catch(...) {}
        return 2;
    }catch(...) {return 3;}
}
