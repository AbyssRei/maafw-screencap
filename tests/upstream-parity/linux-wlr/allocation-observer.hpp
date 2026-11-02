// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Forwarding observations only: identical mmap arguments, return and errno.
static thread_local bool allocation_observing=false;
static thread_local unsigned allocation_map_calls=0;
static thread_local int allocation_map_errno=0,allocation_map_fd=-1;
static thread_local bool allocation_map_failed=false;
extern "C" void* __real_mmap(void*,size_t,int,int,int,off_t);
extern "C" void* __real_mmap64(void*,size_t,int,int,int,off64_t);
static void allocation_map_observe(size_t length,int fd,void* result,int code)
{
    if(allocation_observing && length==268435456 && fd>=0) {
        ++allocation_map_calls;allocation_map_fd=fd;
        allocation_map_errno=code;allocation_map_failed=result==MAP_FAILED;
    }
}
extern "C" void* __wrap_mmap(void* address,size_t length,int prot,int flags,int fd,off_t offset)
{
    void* result=__real_mmap(address,length,prot,flags,fd,offset);
    const int code=errno;
    allocation_map_observe(length,fd,result,code);
    errno=code;return result;
}
extern "C" void* __wrap_mmap64(void* address,size_t length,int prot,int flags,int fd,off64_t offset)
{
    void* result=__real_mmap64(address,length,prot,flags,fd,offset);
    const int code=errno;
    allocation_map_observe(length,fd,result,code);
    errno=code;return result;
}
