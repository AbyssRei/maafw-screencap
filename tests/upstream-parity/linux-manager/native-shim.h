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
struct HostInputInitFailure : std::runtime_error { HostInputInitFailure() : std::runtime_error("controlled host init97") {} };
struct HostInputCreateFailure : std::runtime_error { HostInputCreateFailure() : std::runtime_error("controlled host create98") {} };
struct HostInputState {
    bool enabled=false;
    unsigned outcome=0,native_constructed=0,creation_attempts=0,created=0,initialized=0,inactive=0,destroyed=0;
    unsigned native_closed_before_input=0,host_fds_closed=0,errors=0;
    int capture_observation_error=0;
    std::array<int,2> close_errors{0,0};
    int capture_fd=-1;
};
static HostInputState host_input;
struct UInput : InputBase {
    std::array<int,2> owned{-1,-1};
    UInput(const std::filesystem::path& path,int width,int height) {
        if(!host_input.enabled) { throw std::logic_error("input excluded outside the named host entry"); }
        host_input.native_constructed=1; // Original create_input follows PW construction.
        if(path.string()!="/capture-only-host" || width!=17 || height!=19) { throw std::logic_error("retained host configuration differs"); }
        ++host_input.creation_attempts;
        if(host_input.outcome==3) { throw HostInputCreateFailure(); }
        if(::socketpair(AF_UNIX,SOCK_STREAM|SOCK_CLOEXEC,0,owned.data())!=0) { throw std::runtime_error("host socketpair construction failed"); }
        ++host_input.created;
    }
    UInput(const UInput&)=delete;
    UInput& operator=(const UInput&)=delete;
    ~UInput() override {
        errno=0;
        if(::fcntl(host_input.capture_fd,F_GETFD)==-1 && errno==EBADF) { ++host_input.native_closed_before_input; }
        else { host_input.capture_observation_error=errno;++host_input.errors; }
        for(size_t index=0;index<owned.size();++index) {
            if(::close(owned[index])==0) { ++host_input.host_fds_closed; }
            else { host_input.close_errors[index]=errno;++host_input.errors; }
        }
        ++host_input.destroyed;
    }
    bool init() override {
        ++host_input.initialized;
        if(host_input.outcome==2) { throw HostInputInitFailure(); }
        return host_input.outcome==0;
    }
    void inactive() override { ++host_input.inactive; }
    MaaControllerFeature get_features() const override { return UINT64_MAX; }
};
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
