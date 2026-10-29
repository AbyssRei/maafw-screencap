// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Exact manager bodies with explicitly scripted backend/client boundaries and
// real standard shared owners, filesystem streaming and OpenCV output Mats.
// Input=None only. No Linux daemon, compositor, config JSON or capture claim.
#include "MaaFramework/MaaDef.h"
#include <opencv2/core.hpp>
#include <cstdint>
#include <filesystem>
#include <iomanip>
#include <iostream>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace {
struct Plan { bool client_ok,init_ok,input_throws;int outcome; } plan{};
std::vector<std::string> events;
void event(const char* name) { events.emplace_back(name); }
struct NullLog { template<class T> const NullLog& operator<<(const T&) const { return *this; } } log;
#define LogError log
#define LogFunc log
#define VAR(...) 0
struct LinuxControlUnitConfig {
    MaaLinuxScreencapMethod screencap_method=MaaLinuxScreencapMethod_None;
    MaaLinuxInputMethod input_method=MaaLinuxInputMethod_None;
    std::filesystem::path wlr_socket_path,uinput_path,eis_socket_path;
    int pw_socket_fd=-1;
    uint32_t pw_node_id=0;
    int uinput_screen_width=0,uinput_screen_height=0;
};
struct WaylandClient {
    bool connected_=false;
    WaylandClient() { event("client.create"); }
    ~WaylandClient() { event("client.drop"); }
    bool connect(const std::filesystem::path&) {
        event("client.connect");connected_=plan.client_ok;return connected_;
    }
};
struct ScreencapBase {
    virtual ~ScreencapBase()=default;
    virtual bool init()=0;
    virtual bool screencap(cv::Mat&)=0;
    virtual void inactive()=0;
};
struct Unit : ScreencapBase {
    uint64_t method;
    std::shared_ptr<WaylandClient> client;
    Unit(uint64_t value,std::shared_ptr<WaylandClient> owner={}) : method(value),client(std::move(owner)) { event("capture.create"); }
    ~Unit() override { event("capture.drop"); }
    bool init() override { event("capture.init");return method!=MaaLinuxScreencapMethod_PipeWire || plan.init_ok; }
    bool screencap(cv::Mat& output) override {
        event("capture.call");
        if(method==MaaLinuxScreencapMethod_None || (client && !client->connected_)) { return false; }
        switch(plan.outcome) {
        case 0:return false;
        case 1:output=cv::Mat(1,1,CV_32SC1,cv::Scalar(7));return true;
        case 2:output.release();return true;
        case 3:throw std::runtime_error("scripted backend exception");
        case 4:output=cv::Mat(1,1,CV_32SC1,cv::Scalar(7));return false;
        default:throw std::logic_error("invalid outcome");
        }
    }
    void inactive() override { event("capture.inactive"); }
};
struct NoneScreencap : Unit { NoneScreencap():Unit(MaaLinuxScreencapMethod_None) {} };
struct WlrScreencap : Unit { explicit WlrScreencap(std::shared_ptr<WaylandClient> c):Unit(MaaLinuxScreencapMethod_Wlr,std::move(c)) {} };
struct PipeWireScreencap : Unit { PipeWireScreencap(int,uint32_t):Unit(MaaLinuxScreencapMethod_PipeWire) {} };
struct InputBase {
    virtual ~InputBase()=default;
    virtual bool init() { event("input.init");return true; }
    virtual void inactive() { event("input.inactive"); }
};
struct NoneInput : InputBase {
    NoneInput() { if(plan.input_throws) { throw std::runtime_error("scripted input construction exception"); }event("input.create"); }
    ~NoneInput() override { event("input.drop"); }
};
struct WlrInput : InputBase { explicit WlrInput(std::shared_ptr<WaylandClient>) { throw std::logic_error("input excluded"); } };
struct UInput : InputBase { UInput(const std::filesystem::path&,int,int) { throw std::logic_error("input excluded"); } };
struct EiInput : InputBase { explicit EiInput(const std::filesystem::path&) { throw std::logic_error("input excluded"); } };
struct LinuxControlUnitMgr {
    explicit LinuxControlUnitMgr(const LinuxControlUnitConfig&);
    ~LinuxControlUnitMgr();
    bool connect();bool connected() const;bool request_uuid(std::string&);
    bool screencap(cv::Mat&);bool inactive();
    bool create_screencap();bool create_input();bool create_wl_client();
    const LinuxControlUnitConfig config_;
    bool connected_=false;
    std::shared_ptr<InputBase> input_=nullptr;
    std::shared_ptr<ScreencapBase> screencap_=nullptr;
    std::shared_ptr<WaylandClient> wl_client_=nullptr;
};
#include "linux-manager-source.inc"
#undef LogError
#undef LogFunc
#undef VAR
std::string bytes(const std::string& input) {
    constexpr char digits[]="0123456789abcdef";
    std::string result;
    for(unsigned char c:input) { result.push_back(digits[c>>4]);result.push_back(digits[c&15]); }
    return result;
}
std::string decode(const char* input) {
    const std::string text(input);if(text.size()%2) { throw std::logic_error("odd path hex"); }
    std::string result;
    for(size_t i=0;i<text.size();i+=2) { result.push_back(static_cast<char>(std::stoul(text.substr(i,2),nullptr,16))); }
    return result;
}
}
int main(int argc,char** argv) {
    try {
        if(argc!=9) { throw std::logic_error("method fd node client_ok init_ok input_throws outcome path_hex"); }
        LinuxControlUnitConfig config;
        config.screencap_method=std::stoull(argv[1]);config.pw_socket_fd=std::stoi(argv[2]);
        const auto node=std::stoull(argv[3]);if(node>UINT32_MAX) { throw std::logic_error("node overflow"); }
        config.pw_node_id=static_cast<uint32_t>(node);config.wlr_socket_path=decode(argv[8]);
        plan={std::stoi(argv[4])!=0,std::stoi(argv[5])!=0,std::stoi(argv[6])!=0,std::stoi(argv[7])};
        {
            LinuxControlUnitMgr manager(config);
            std::cout<<"initial="<<manager.connected()<<'\n';
            std::string uuid="unchanged";
            std::cout<<"uuid_success="<<manager.request_uuid(uuid)<<"\nuuid="<<bytes(uuid)<<'\n';
            cv::Mat output(1,1,CV_32SC1,cv::Scalar(99));
            for(int attempt=0;attempt<2;++attempt) {
                int connected=2,captured=2;
                try { connected=manager.connect(); }catch(const std::runtime_error&) {}
                std::cout<<"connect"<<attempt<<'='<<connected<<"\nconnected"<<attempt<<'='<<manager.connected()<<'\n';
                try { captured=manager.screencap(output); }catch(const std::runtime_error&) {}
                std::cout<<"capture"<<attempt<<'='<<captured<<"\noutput"<<attempt<<'='<<(output.empty()?0:output.at<int>(0,0))<<'\n';
                std::cout<<"inactive"<<attempt<<'='<<manager.inactive()<<'\n';
            }
        }
        std::cout<<"events=";
        for(size_t i=0;i<events.size();++i) { if(i) { std::cout<<','; }std::cout<<events[i]; }
        std::cout<<'\n';return 0;
    }catch(const std::exception& error) { std::cerr<<"Linux manager model failure: "<<error.what()<<'\n';return 1; }
}
