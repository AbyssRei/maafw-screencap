// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Test-only ELF interposition. The real context/core/error/teardown paths run;
// only the owned not-yet-running loop's exact offered-socket add_io is rejected.
// Product code, properties, connection selection and cleanup are unchanged.
#include "fd-fault.h"
#include <pipewire/pipewire.h>
#include <cerrno>
#include <climits>
#include <dlfcn.h>

namespace {
struct State {
    int32_t fd=-1;
    bool armed=false;
    uint32_t attempts=0,rejections=0;
    int32_t native_error=0,control_error=0;
};
thread_local State state;
struct Fault {
    spa_loop_utils_methods methods;
    decltype(spa_loop_utils_methods::add_io) original;
};
thread_local Fault* active=nullptr;
spa_source* reject(void* data,int fd,uint32_t mask,bool close,
                   spa_source_io_func_t callback,void* user) noexcept {
    if(!active) { state.control_error=EINVAL;errno=EINVAL;return nullptr; }
    if(fd!=state.fd) { return active->original(data,fd,mask,close,callback,user); }
    if(state.rejections==UINT32_MAX) { state.control_error=EOVERFLOW;errno=EOVERFLOW;return nullptr; }
    ++state.rejections;
    errno=ENOMEM;
    return nullptr;
}
using Connect=pw_core*(*)(pw_context*,int,pw_properties*,size_t);
Connect real_connect() noexcept {
    // POSIX dlsym supplies the C function pointer. Keep the selected native
    // library loaded for the process; the interposer never owns its objects.
    static const auto function=reinterpret_cast<Connect>(dlsym(RTLD_NEXT,"pw_context_connect_fd"));
    return function;
}
}
extern "C" int32_t mfs_pw_fd_fault_arm(int32_t fd) noexcept {
    if(fd<0 || active || state.armed) { return EINVAL; }
    state=State{};
    state.fd=fd;
    state.armed=true;
    return 0;
}
extern "C" uint32_t mfs_pw_fd_fault_attempts() noexcept { return state.attempts; }
extern "C" uint32_t mfs_pw_fd_fault_rejections() noexcept { return state.rejections; }
extern "C" int32_t mfs_pw_fd_fault_native_error() noexcept { return state.native_error; }
extern "C" int32_t mfs_pw_fd_fault_control_error() noexcept { return state.control_error; }

extern "C" MFS_PW_FD_EXPORT pw_core* pw_context_connect_fd(
    pw_context* context,int fd,pw_properties* properties,size_t user_size) {
    const auto connect=real_connect();
    if(!connect) { state.control_error=ENOSYS;errno=ENOSYS;return nullptr; }
    if(fd!=state.fd) { return connect(context,fd,properties,user_size); }
    if(state.attempts==UINT32_MAX) { state.control_error=EOVERFLOW;errno=EOVERFLOW;return nullptr; }
    ++state.attempts;
    if(!state.armed) { return connect(context,fd,properties,user_size); }
    state.armed=false; // one controlled attempt, never an automatic retry
    const auto loop=pw_context_get_main_loop(context);
    if(active || !loop || !loop->utils || !loop->utils->iface.cb.funcs) {
        state.control_error=EINVAL;errno=EINVAL;return nullptr;
    }
    auto* interface=&loop->utils->iface;
    const auto* original=interface->cb.funcs;
    Fault fault{*static_cast<const spa_loop_utils_methods*>(original),nullptr};
    fault.original=fault.methods.add_io;
    if(!fault.original) { state.control_error=ENOSYS;errno=ENOSYS;return nullptr; }
    fault.methods.add_io=reject;
    active=&fault;
    interface->cb.funcs=&fault.methods;
    // Native connect synchronously consumes this owned loop utility table.
    // No exception is introduced; all other methods and callback data stay real.
    const auto core=connect(context,fd,properties,user_size);
    const int error=errno;
    interface->cb.funcs=original; // restore before caller cleanup can begin
    active=nullptr;
    if(!core && state.native_error==0) { state.native_error=error; }
    errno=error;
    return core;
}
