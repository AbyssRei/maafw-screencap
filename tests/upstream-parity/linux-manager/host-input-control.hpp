// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Included inside the selected original namespace. Only host construction/init
// is controlled; real socketpairs witness independent ownership, never input.
#ifndef MFS_HOST_INPUT_BASE
#define MFS_HOST_INPUT_BASE InputBase
#define MFS_HOST_INPUT_DEFAULT_BASE
#endif
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
struct UInput : MFS_HOST_INPUT_BASE {
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
#ifdef MFS_HOST_INPUT_DEFAULT_BASE
#undef MFS_HOST_INPUT_DEFAULT_BASE
#undef MFS_HOST_INPUT_BASE
#endif
