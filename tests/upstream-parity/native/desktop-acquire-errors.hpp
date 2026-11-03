// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "d3d-lifetime.h"
namespace {
std::array<uint64_t,12> error_lifetime(uint32_t index){
    std::array<uint64_t,12> row{};const auto code=mfs_d3d_lifetime_read(index,row.data(),row.size());
    if(code){throw std::runtime_error("Acquire lifetime read HRESULT="+std::to_string(code));}return row;
}
void error_lifetimes(std::ostream& out,uint32_t cycle,uint32_t phase){
    for(uint32_t kind=0;kind<4;++kind){out<<cycle<<'\t'<<phase<<'\t'<<kind;for(auto value:error_lifetime(cycle*4+kind)){out<<'\t'<<value;}out<<'\n';}out.flush();
}
void error_state(std::ostream& out,uint32_t cycle,uint32_t phase,const std::array<uint64_t,12>& state){
    out<<cycle<<'\t'<<phase;for(auto value:state){out<<'\t'<<value;}out<<'\n';out.flush();
}
bool error_wait(uint32_t cycle,uint32_t phase,bool access_lost){
    const auto deadline=GetTickCount64()+2000;
    for(;;){bool done=true;for(uint32_t kind=0;kind<4;++kind){
        const bool released=phase==3 || (access_lost && phase>=1 && kind>=2);
        done&=error_lifetime(cycle*4+kind)[4]==uint64_t(released);
    }if(done || GetTickCount64()>=deadline)return done;Sleep(1);}
}
int acquire_errors(const std::filesystem::path& root){
    try{
        if(!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root))throw std::runtime_error("Acquire errors root must be empty");
        if(mfs_d3d_lifetime_version()!=1)throw std::runtime_error("Acquire observer ABI differs");
        std::array<uint32_t,6> widths{};if(mfs_d3d_lifetime_widths(widths.data(),widths.size()) || widths!=std::array<uint32_t,6>{8,8,8,4,4,4})throw std::runtime_error("Acquire native scalar widths differ");
        const std::array<HRESULT,3> errors{DXGI_ERROR_ACCESS_LOST,DXGI_ERROR_WAIT_TIMEOUT,DXGI_ERROR_DEVICE_REMOVED};
        std::ofstream abi(root/"abi.tsv",std::ios::binary),calls(root/"calls.tsv",std::ios::binary),states(root/"states.tsv",std::ios::binary),life(root/"lifetimes.tsv",std::ios::binary);
        for(auto* out:{&abi,&calls,&states,&life})out->exceptions(std::ios::badbit|std::ios::failbit);
        abi<<1;for(auto value:widths)abi<<'\t'<<value;for(auto code:errors)abi<<'\t'<<static_cast<uint32_t>(code);abi<<'\n';abi.close();
        calls<<"cycle\tmode\tcode\tempty\torder\tacquires\tnative_acquires\tinjected\tcopies\tmaps\tunmaps\treleases\tconversions\ttimeout\tviolation\tcaller\tacquire_duplication\tbegin_fault\tend_fault\tend_observation\tend_recovery\tend_recovery_observation\tend_close\tend_close_observation\n";
        states<<"cycle\tphase\tslots\tmonitor\tjust_initialized\twidth\theight\tdevice\tcontext\tfactory\tadapter\toutput\tduplication\tstaging\n";
        life<<"cycle\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr\n";
        struct Saved{std::string name;cv::Mat image;};std::vector<Saved> held;held.reserve(40);
        for(uint32_t cycle=0;cycle<20;++cycle){
            const auto mode=cycle%3;const bool access_lost=mode==0;Fixture fixture;
            if(mfs_print_thread_fixture(96,64,0,0,0,0,MFS_RENDER_DISABLE_TRANSITIONS|MFS_RENDER_TOPMOST,&fixture.value) || mfs_desktop_fixture_inside_updates(fixture.value.owner))throw std::runtime_error("Acquire errors fixture failed");
            Source source(static_cast<HWND>(fixture.value.hwnd));auto warm=source.screencap();if(!warm || !source.warm())throw std::runtime_error("Acquire errors warm state failed");
            auto image=target(*warm,static_cast<HWND>(fixture.value.hwnd));save(root,std::to_string(cycle)+"-warm",image);held.push_back({std::to_string(cycle)+"-warm-held",image});
            const auto before=source.state();const auto objects=source.observed_objects();
            for(uint32_t kind=0;kind<4;++kind){const auto code=mfs_d3d_lifetime_attach(objects[kind],kind,cycle*4+kind);if(code)throw std::runtime_error("Acquire native attach HRESULT="+std::to_string(code));}
            error_state(states,cycle,0,before);error_lifetimes(life,cycle,0);
            mfs_dup_map::Trace trace;trace.fault=mode+3;std::optional<cv::Mat> rejected;
            const auto begin=mfs_d3d_lifetime_clock();{mfs_dup_map::Scope scope(trace);rejected=source.screencap();}const auto end=mfs_d3d_lifetime_clock();
            const auto failed=source.state();error_state(states,cycle,1,failed);const bool fault_owners=error_wait(cycle,1,access_lost);const auto observed=mfs_d3d_lifetime_clock();error_lifetimes(life,cycle,1);
            if(rejected || trace.order!=1 || trace.acquires!=1 || trace.native_acquires || !trace.injected_acquire || trace.acquired!=errors[mode]
                || trace.copies || trace.maps || trace.unmaps || trace.releases || trace.conversions || trace.timeout!=2000 || trace.violation || trace.held || trace.mapped || trace.thread!=GetCurrentThreadId()
                || trace.duplication!=before[10] || trace.context || trace.resource || !fault_owners)throw std::runtime_error("controlled Acquire result/call boundary or old-owner lifetime differs");
            auto recovered=source.screencap();if(!recovered || !source.warm())throw std::runtime_error("Acquire errors next-operation capture failed");const auto recovered_at=mfs_d3d_lifetime_clock();
            auto recovered_image=target(*recovered,static_cast<HWND>(fixture.value.hwnd));save(root,std::to_string(cycle)+"-recovery",recovered_image);held.push_back({std::to_string(cycle)+"-recovery-held",recovered_image});
            error_state(states,cycle,2,source.state());const bool recovery_owners=error_wait(cycle,2,access_lost);const auto recovery_observed=mfs_d3d_lifetime_clock();error_lifetimes(life,cycle,2);
            if(!recovery_owners)throw std::runtime_error("recovery old-owner lifetime differs");
            source.close();const auto closed=mfs_d3d_lifetime_clock();error_state(states,cycle,3,source.state());const bool closed_owners=error_wait(cycle,3,access_lost);const auto close_observed=mfs_d3d_lifetime_clock();error_lifetimes(life,cycle,3);
            if(!closed_owners)throw std::runtime_error("closed old-owner lifetime differs");
            for(uint32_t kind=0;kind<4;++kind)if(error_lifetime(cycle*4+kind)[4]!=1)throw std::runtime_error("observed old native owner did not release");
            calls<<cycle<<'\t'<<mode<<'\t'<<static_cast<uint32_t>(trace.acquired)<<"\t1\t"<<trace.order<<'\t'<<trace.acquires<<'\t'<<trace.native_acquires<<'\t'<<trace.injected_acquire<<'\t'<<trace.copies<<'\t'<<trace.maps<<'\t'<<trace.unmaps<<'\t'<<trace.releases<<'\t'<<trace.conversions<<'\t'<<trace.timeout<<'\t'<<trace.violation<<'\t'<<GetCurrentThreadId()<<'\t'<<trace.duplication<<'\t'<<begin<<'\t'<<end<<'\t'<<observed<<'\t'<<recovered_at<<'\t'<<recovery_observed<<'\t'<<closed<<'\t'<<close_observed<<'\n';calls.flush();fixture.close();
        }
        for(const auto& image:held)save(root,image.name,image.image);
        std::cout<<"original DesktopDup acquire_errors cycles=20 public_calls=60 controlled_failures=20 access_lost=7 retained_other_errors=13 recoveries=20 observed_old_objects=80 retained_cpu=40 physical_device_faults=0 full_P1_acceptance=0 passed\n";return 0;
    }catch(const std::exception& error){std::cerr<<"Acquire errors: "<<error.what()<<'\n';return 1;}
}
}
