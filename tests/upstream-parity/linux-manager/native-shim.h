// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Include INSIDE the original private source namespace after its real selected
// capture class/NoneScreencap. Exact manager bodies; Input=None only. Unselected
// platform/input constructors fail explicitly rather than fabricate capture.
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
};
struct NoneInput : InputBase {};
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
#ifndef LogFunc
#define LogFunc LogTrace
#endif
#include "linux-manager-source.inc"
