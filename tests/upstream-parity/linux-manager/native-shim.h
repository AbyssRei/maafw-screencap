// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Include INSIDE the original private source namespace after its real selected
// capture class/NoneScreencap. Exact manager bodies; normal entries use None.
// The explicitly enabled host-input entry below is a scripted initialization
// boundary with owned real socketpair resources, never device input support.
#include "linux-manager-config.inc"
#if defined(MFS_MANAGER_WLR)
struct PipeWireScreencap : ScreencapBase {
    PipeWireScreencap(int,uint32_t) { throw std::logic_error("PipeWire excluded from Wlr source adapter"); }
    bool screencap(cv::Mat&) override { return false; }
};
#else
struct WaylandClient {
    bool connect(const std::filesystem::path&) { throw std::logic_error("Wayland excluded from PipeWire source adapter"); }
};
struct WlrScreencap : ScreencapBase {
    explicit WlrScreencap(std::shared_ptr<WaylandClient>) { throw std::logic_error("Wlr excluded from PipeWire source adapter"); }
    bool screencap(cv::Mat&) override { return false; }
};
#endif
struct InputBase {
    virtual ~InputBase()=default;
    virtual bool init() { return true; }
    virtual void inactive() {}
    virtual MaaControllerFeature get_features() const { return 0; }
};
struct NoneInput : InputBase {};
struct WlrInput : InputBase { explicit WlrInput(std::shared_ptr<WaylandClient>) { throw std::logic_error("input excluded"); } };
#if defined(MFS_MANAGER_HOST_INPUT)
#include "host-input-control.hpp"
#else
struct UInput : InputBase { UInput(const std::filesystem::path&,int,int) { throw std::logic_error("input excluded"); } };
#endif
struct EiInput : InputBase { explicit EiInput(const std::filesystem::path&) { throw std::logic_error("input excluded"); } };
struct LinuxControlUnitMgr {
    explicit LinuxControlUnitMgr(const LinuxControlUnitConfig&);
    ~LinuxControlUnitMgr();
    bool connect();bool connected() const;bool request_uuid(std::string&);
    bool screencap(cv::Mat&);bool inactive();
    MaaControllerFeature get_features() const;
    bool create_screencap();bool create_input();bool create_wl_client();
    const LinuxControlUnitConfig config_;
    bool connected_=false;
    std::shared_ptr<InputBase> input_=nullptr;
    std::shared_ptr<ScreencapBase> screencap_=nullptr;
    std::shared_ptr<WaylandClient> wl_client_=nullptr;
};
#ifndef LogFunc
#define LogFunc LogTrace
#endif
#include "linux-manager-source.inc"
#if defined(MFS_MANAGER_HOST_INPUT)
#include "linux-manager-features.inc"
#endif
