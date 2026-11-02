// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include <pthread.h>
#include <signal.h>

enum class ResourceFault { None, Memfd, Truncate };
struct ResourceObservation {
    unsigned calls=0;
    int fd=-1,error=0;
    rlimit before{},after{};
    unsigned mask_before=0,mask_restored=0,pending=0,consumed=0,action_same=0;
};
static thread_local ResourceFault resource_fault=ResourceFault::None;
static thread_local ResourceObservation resource_observation;
static thread_local const char* resource_stage="initial";
static void resource_check(bool value) noexcept {
    if(!value) {
        constexpr char prefix[]="owned resource guard premise failed: ";
        (void)write(STDERR_FILENO,prefix,sizeof(prefix)-1);
        (void)write(STDERR_FILENO,resource_stage,std::strlen(resource_stage));
        (void)write(STDERR_FILENO,"\n",1);
        _exit(125);
    }
}
static bool resource_same_mask(const sigset_t& a,const sigset_t& b) noexcept {
    for(int signal=1;signal<=SIGRTMAX;++signal) {
        if(sigismember(&a,signal)!=sigismember(&b,signal)) { return false; }
    }
    return true;
}
static bool resource_same_action(const struct sigaction& a,const struct sigaction& b) noexcept {
    if(a.sa_flags!=b.sa_flags || !resource_same_mask(a.sa_mask,b.sa_mask)) { return false; }
    return (a.sa_flags&SA_SIGINFO) ? a.sa_sigaction==b.sa_sigaction : a.sa_handler==b.sa_handler;
}
template<class Call> static int resource_limited_call(ResourceFault kind,int fd,Call call) noexcept {
    auto& out=resource_observation;
    ++out.calls;out.fd=fd;
    const auto which=kind==ResourceFault::Memfd ? RLIMIT_NOFILE : RLIMIT_FSIZE;
    sigset_t original{},selected{},pending{},current{};
    struct sigaction action{},after_action{};
    resource_stage="signal-context";
    resource_check(pthread_sigmask(SIG_SETMASK,nullptr,&original)==0);
    resource_check(sigaction(SIGXFSZ,nullptr,&action)==0);
    out.mask_before=static_cast<unsigned>(sigismember(&original,SIGXFSZ));
    resource_check(out.mask_before<=1 && sigemptyset(&selected)==0 && sigaddset(&selected,SIGXFSZ)==0);
    if(kind==ResourceFault::Truncate) {
        resource_check(pthread_sigmask(SIG_BLOCK,&selected,nullptr)==0);
        resource_check(sigpending(&pending)==0 && sigismember(&pending,SIGXFSZ)==0);
    }
    resource_stage="limit-selection";
    resource_check(getrlimit(which,&out.before)==0 && out.before.rlim_cur>0);
    resource_check(kind!=ResourceFault::Truncate || out.before.rlim_cur>=268435456);
    auto limit=out.before;limit.rlim_cur=0;
    resource_check(setrlimit(which,&limit)==0);
    const int result=call();const int code=errno;
    // Restore first, even when the real syscall produced an unexpected result.
    resource_stage="limit-restoration";
    resource_check(setrlimit(which,&out.before)==0 && getrlimit(which,&out.after)==0);
    resource_check(out.before.rlim_cur==out.after.rlim_cur && out.before.rlim_max==out.after.rlim_max);
    if(kind==ResourceFault::Truncate) {
        resource_stage="new-SIGXFSZ-consumption";
        resource_check(sigpending(&pending)==0);
        out.pending=static_cast<unsigned>(sigismember(&pending,SIGXFSZ));
        if(out.pending==1) {
            const timespec zero{};
            out.consumed=sigtimedwait(&selected,nullptr,&zero)==SIGXFSZ;
        }
        resource_check(out.pending==1 && out.consumed==1 && sigpending(&pending)==0
            && sigismember(&pending,SIGXFSZ)==0);
        resource_stage="thread-mask-restoration";
        resource_check(pthread_sigmask(SIG_SETMASK,&original,nullptr)==0);
    }
    resource_check(pthread_sigmask(SIG_SETMASK,nullptr,&current)==0 && sigaction(SIGXFSZ,nullptr,&after_action)==0);
    out.mask_restored=resource_same_mask(original,current);
    out.action_same=resource_same_action(action,after_action);
    out.error=code;
    resource_stage="real-kernel-outcome";
    resource_check(out.mask_restored && out.action_same && result==-1
        && code==(kind==ResourceFault::Memfd ? EMFILE : EFBIG));
    errno=code;return result;
}
extern "C" int __real_memfd_create(const char*,unsigned int);
extern "C" int __real_ftruncate(int,off_t);
extern "C" int __real_ftruncate64(int,off64_t);
extern "C" int __wrap_memfd_create(const char* name,unsigned int flags) {
    if(resource_fault==ResourceFault::Memfd && name && std::strncmp(name,"maafw-wl-buffer-",16)==0) {
        return resource_limited_call(ResourceFault::Memfd,-1,[&]() noexcept { return __real_memfd_create(name,flags); });
    }
    return __real_memfd_create(name,flags);
}
extern "C" int __wrap_ftruncate(int fd,off_t length) {
    if(resource_fault==ResourceFault::Truncate && length==268435456) {
        return resource_limited_call(ResourceFault::Truncate,fd,[&]() noexcept { return __real_ftruncate(fd,length); });
    }
    return __real_ftruncate(fd,length);
}
extern "C" int __wrap_ftruncate64(int fd,off64_t length) {
    if(resource_fault==ResourceFault::Truncate && length==268435456) {
        return resource_limited_call(ResourceFault::Truncate,fd,[&]() noexcept { return __real_ftruncate64(fd,length); });
    }
    return __real_ftruncate64(fd,length);
}
