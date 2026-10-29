// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "acceptance-evidence.h"
#include <cstdint>
#include <climits>
#include <libproc.h>
#include <mach/mach.h>
#include <stdexcept>
#include <sys/proc_info.h>
#include <unistd.h>
#include <vector>
inline MfsMacResources mfs_mac_observe_resources() {
    auto check=[](bool value,const char* message) { if(!value) { throw std::runtime_error(message); } };
    MfsMacResources result{};
    const int bytes=proc_pidinfo(getpid(),PROC_PIDLISTFDS,0,nullptr,0);
    check(bytes>=0,"cannot query own descriptor table");
    const auto capacity=static_cast<size_t>(bytes)/sizeof(proc_fdinfo)+32;
    check(capacity<=static_cast<size_t>(INT_MAX)/sizeof(proc_fdinfo),"descriptor table narrowing");
    std::vector<proc_fdinfo> fds(capacity);
    const int used=proc_pidinfo(getpid(),PROC_PIDLISTFDS,0,fds.data(),static_cast<int>(fds.size()*sizeof(proc_fdinfo)));
    check(used>=0 && static_cast<size_t>(used)%sizeof(proc_fdinfo)==0,"cannot read own descriptor table");
    result.descriptors=static_cast<size_t>(used)/sizeof(proc_fdinfo);
    thread_act_array_t threads=nullptr;mach_msg_type_number_t thread_count=0;
    check(task_threads(mach_task_self(),&threads,&thread_count)==KERN_SUCCESS,"cannot enumerate own threads");
    result.threads=thread_count;
    bool released=true;
    for(mach_msg_type_number_t index=0;index<thread_count;++index) {
        if(mach_port_deallocate(mach_task_self(),threads[index])!=KERN_SUCCESS) { released=false; }
    }
    if(vm_deallocate(mach_task_self(),reinterpret_cast<vm_address_t>(threads),
        static_cast<vm_size_t>(thread_count)*sizeof(thread_t))!=KERN_SUCCESS) { released=false; }
    check(released,"cannot release thread observation rights/storage");
    mach_port_name_array_t names=nullptr;mach_port_type_array_t types=nullptr;
    mach_msg_type_number_t name_count=0,type_count=0;
    check(mach_port_names(mach_task_self(),&names,&name_count,&types,&type_count)==KERN_SUCCESS,
        "cannot enumerate own Mach port names");
    result.ports=name_count;
    const auto names_result=vm_deallocate(mach_task_self(),reinterpret_cast<vm_address_t>(names),
        static_cast<vm_size_t>(name_count)*sizeof(mach_port_name_t));
    const auto types_result=vm_deallocate(mach_task_self(),reinterpret_cast<vm_address_t>(types),
        static_cast<vm_size_t>(type_count)*sizeof(mach_port_type_t));
    check(names_result==KERN_SUCCESS && types_result==KERN_SUCCESS,"cannot release port observation storage");
    return result;
}
