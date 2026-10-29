// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "dma-fixture.h"
#include <array>
#include <cerrno>
#include <climits>
#include <cstddef>
#include <cstring>
#include <fcntl.h>
#include <linux/dma-buf.h>
#include <linux/dma-heap.h>
#include <linux/udmabuf.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>
namespace {
struct Fd { int value;~Fd() { if(value>=0) { close(value); } } };
int sync(int fd,bool start) noexcept {
    dma_buf_sync value{};
    value.flags=DMA_BUF_SYNC_WRITE|(start ? DMA_BUF_SYNC_START : DMA_BUF_SYNC_END);
    return ioctl(fd,DMA_BUF_IOCTL_SYNC,&value)<0 ? errno : 0;
}
}
extern "C" int32_t mfs_pw_dma_create(uint32_t exporter,uint64_t bytes) noexcept {
    const long page=sysconf(_SC_PAGESIZE);
    if(page<=0 || bytes==0 || bytes>INT64_MAX || bytes%static_cast<uint64_t>(page)!=0) { return -EINVAL; }
    if(exporter==1) {
        Fd device{open("/dev/udmabuf",O_RDWR|O_CLOEXEC)};
        if(device.value<0) { return -errno; }
        Fd backing{memfd_create("mfs-p1-dma-backing",MFD_CLOEXEC|MFD_ALLOW_SEALING)};
        if(backing.value<0) { return -errno; }
        if(ftruncate(backing.value,static_cast<off_t>(bytes))<0 ||
           fcntl(backing.value,F_ADD_SEALS,F_SEAL_SHRINK|F_SEAL_GROW)<0) { return -errno; }
        udmabuf_create create{static_cast<uint32_t>(backing.value),UDMABUF_FLAGS_CLOEXEC,0,bytes};
        const int fd=ioctl(device.value,UDMABUF_CREATE,&create);
        return fd<0 ? -errno : fd;
    }
    if(exporter==2) {
        Fd device{open("/dev/dma_heap/system",O_RDWR|O_CLOEXEC)};
        if(device.value<0) { return -errno; }
        dma_heap_allocation_data allocate{bytes,0,O_RDWR|O_CLOEXEC,0};
        if(ioctl(device.value,DMA_HEAP_IOCTL_ALLOC,&allocate)<0) { return -errno; }
        if(allocate.fd>INT_MAX) { return -EOVERFLOW; } // kernel descriptor ABI is int
        return static_cast<int32_t>(allocate.fd);
    }
    return -EINVAL;
}
extern "C" int32_t mfs_pw_dma_write(int32_t fd,const uint8_t* input,uint64_t bytes) noexcept {
    if(fd<0 || !input || bytes==0 || bytes>INT64_MAX) { return EINVAL; }
    struct stat info{};
    if(fstat(fd,&info)<0) { return errno; }
    if(info.st_size<0 || bytes>static_cast<uint64_t>(info.st_size)) { return EINVAL; }
    void* pointer=mmap(nullptr,static_cast<size_t>(bytes),PROT_READ|PROT_WRITE,MAP_SHARED,fd,0);
    if(pointer==MAP_FAILED) { return errno; }
    int result=sync(fd,true);
    if(result==0) {
        std::memcpy(pointer,input,static_cast<size_t>(bytes));
        result=sync(fd,false);
    }
    if(munmap(pointer,static_cast<size_t>(bytes))<0 && result==0) { result=errno; }
    return result;
}
extern "C" uint64_t mfs_pw_dma_abi(uint32_t index) noexcept {
    constexpr std::array<uint64_t,7> values={sizeof(dma_buf_sync),alignof(dma_buf_sync),
        offsetof(dma_buf_sync,flags),DMA_BUF_IOCTL_SYNC,DMA_BUF_SYNC_READ,DMA_BUF_SYNC_START,DMA_BUF_SYNC_END};
    return index<values.size() ? values[index] : UINT64_MAX;
}
