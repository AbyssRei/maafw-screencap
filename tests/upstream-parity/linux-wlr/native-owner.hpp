// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "Manager/LinuxControlUnitMgr.h"
#include "MaaControlUnit/WlRootsControlUnitAPI.h"
MAA_CTRL_UNIT_NS_BEGIN
#include "../linux-manager/native-abi-input.hpp"
class NoneInput : public FixtureInput {};
class PipeWireScreencap : public ScreencapBase {
public:
    PipeWireScreencap(int,uint32_t) {throw std::logic_error("PipeWire excluded from Wlr context");}
    bool screencap(cv::Mat&) override {throw std::logic_error("PipeWire excluded");}
};
class UInput : public FixtureInput {
public:
    UInput(const std::filesystem::path&,int,int) {throw std::logic_error("UInput excluded");}
};
class EiInput : public FixtureInput {
public:
    explicit EiInput(const std::filesystem::path&) {throw std::logic_error("LibEi excluded");}
};
struct HostInitFailure : std::runtime_error {HostInitFailure() : std::runtime_error("controlled Wlr host init97") {}};
struct HostState {
    unsigned outcome=0,created=0,initialized=0,inactive=0,destroyed=0;
    long create_refs=0,destroy_refs=0;
    unsigned connected=0,closed=0,errors=0;
    unsigned snapshot_mask=0;
    std::weak_ptr<WaylandClient> client;
    std::array<std::weak_ptr<void>,5> snapshots;
} host;
// Capture-only host boundary: reproduce original WlrInput constructor's shared
// client/proxy ownership, controlling init rather than creating input devices.
class WlrInput : public FixtureInput {
    std::shared_ptr<WaylandClient> client_;
    std::shared_ptr<wl_shm> shm_;
    std::shared_ptr<wl_seat> seat_;
    std::shared_ptr<wl_output> output_;
    std::shared_ptr<zwlr_virtual_pointer_manager_v1> pointer_;
    std::shared_ptr<zwp_virtual_keyboard_manager_v1> keyboard_;
    std::array<int,2> owned_{-1,-1};
public:
    explicit WlrInput(std::shared_ptr<WaylandClient> client)
        : client_(std::move(client)),shm_(client_->get_shm()),seat_(client_->get_seat()),output_(client_->get_output()),
          pointer_(client_->get_virtual_pointer_manager()),keyboard_(client_->get_virtual_keyboard_manager()) {
        require(client_->connected(),"host received disconnected native client");
        require(shm_ && output_ && pointer_ && keyboard_,"host snapshot missing");
        require(::socketpair(AF_UNIX,SOCK_STREAM|SOCK_CLOEXEC,0,owned_.data())==0,"host socketpair failed");
        host.client=client_;host.snapshots={shm_,seat_,output_,pointer_,keyboard_};
        // Original construction permits an absent seat. This headless fixture
        // supplies no host seat/device; retain that empty slot unchanged.
        for(size_t index=0;index<host.snapshots.size();++index) {
            if(!host.snapshots[index].expired()) {host.snapshot_mask|=1U<<index;}
        }
        host.create_refs=client_.use_count();++host.created;
    }
    ~WlrInput() override {
        host.destroy_refs=client_.use_count();
        host.connected=client_->connected();
        if(host.destroy_refs!=1 || !host.connected) {++host.errors;}
        for(int fd:owned_) {if(::close(fd)==0) {++host.closed;} else {++host.errors;}}
        ++host.destroyed;
        // Snapshot members end before client_, as in original WlrInput. Do not
        // close the shared client while any input/capture snapshot survives.
    }
    bool init() override {++host.initialized;if(host.outcome==2) {throw HostInitFailure();}return host.outcome==0;}
    void inactive() override {++host.inactive;}
    MaaControllerFeature get_features() const override {return UINT64_MAX;}
};
#include "linux-manager-source.inc"
#include "linux-manager-features.inc"
#include "linux-manager-info.inc"
#include "../linux-manager/native-abi-actions.hpp"
MAA_CTRL_UNIT_NS_END
