// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Only explicit isolated consumers preload this library. Real dequeue/queue,
// FD, chunk, producer and stream flags are retained. The original mapped data
// pointer is hidden during each real MemFd lease, then restored before queue.
// This is controlled branch coverage, not an organically NULL native pointer.
#include "memfd-probe.h"
#include <pipewire/pipewire.h>
#include <array>
#include <cerrno>
#include <climits>
#include <cstdlib>
#include <dlfcn.h>
#include <fcntl.h>
#include <mutex>

namespace {
struct Lease {
    pw_stream* stream=nullptr;
    pw_buffer* buffer=nullptr;
    spa_data* plane=nullptr;
    spa_data saved{};
    spa_chunk chunk{};
};
struct State {
    std::mutex mutex;
    bool armed=false;
    std::array<Lease,64> leases{};
    std::array<uint64_t,4> counts{};
};
State state;
template<typename Function> Function next(const char* name) noexcept {
    const auto pointer=dlsym(RTLD_NEXT,name);
    if(!pointer) std::abort(); // no fabricated success without the native API
    return reinterpret_cast<Function>(pointer);
}
bool same(const Lease& lease) noexcept {
    const auto& p=*lease.plane;
    const auto& s=lease.saved;
    return p.type==s.type && p.flags==s.flags && p.fd==s.fd &&
        p.mapoffset==s.mapoffset && p.maxsize==s.maxsize && p.data==nullptr &&
        p.chunk==s.chunk && p.chunk->offset==lease.chunk.offset &&
        p.chunk->size==lease.chunk.size && p.chunk->stride==lease.chunk.stride &&
        p.chunk->flags==lease.chunk.flags;
}
void restore(Lease& lease) noexcept {
    if(!same(lease) || fcntl(static_cast<int>(lease.saved.fd),F_GETFD)<0)
        ++state.counts[3];
    // Even an assertion failure must restore the native pointer before requeue
    // or stream destruction. Neither this fixture nor the consumer owns the FD.
    lease.plane->data=lease.saved.data;
    lease={};
    ++state.counts[1];
    --state.counts[2];
}
}
extern "C" int32_t mfs_pw_memfd_probe_arm() noexcept {
    std::lock_guard lock(state.mutex);
    if(state.armed || state.counts[2]) return -1;
    state.counts={};
    state.armed=true;
    return 0;
}
extern "C" uint64_t mfs_pw_memfd_probe_read(uint32_t field) noexcept {
    std::lock_guard lock(state.mutex);
    return field<state.counts.size() ? state.counts[field] : UINT64_MAX;
}
extern "C" int32_t mfs_pw_memfd_probe_disarm() noexcept {
    std::lock_guard lock(state.mutex);
    if(!state.armed || state.counts[2]) return -1;
    state.armed=false;
    return state.counts[3] ? -1 : 0;
}
extern "C" MFS_PW_MEMFD_EXPORT pw_buffer* pw_stream_dequeue_buffer(pw_stream* stream) try {
    static const auto native=next<decltype(&pw_stream_dequeue_buffer)>("pw_stream_dequeue_buffer");
    pw_buffer* buffer=native(stream);
    if(!buffer) return nullptr;
    const int saved_errno=errno;
    {
        std::lock_guard lock(state.mutex);
        if(state.armed) {
            ++state.counts[0];
            auto* b=buffer->buffer;
            auto* p=b && b->n_datas==1 && b->datas ? b->datas : nullptr;
            Lease* available=nullptr;
            for(auto& lease:state.leases) {
                if(lease.buffer==buffer) { ++state.counts[3]; available=nullptr; break; }
                if(!lease.buffer && !available) available=&lease;
            }
            if(!p || !available || !p->data || !p->chunk ||
               p->type!=SPA_DATA_MemFd || !(p->flags & SPA_DATA_FLAG_READABLE) ||
               p->fd<0 || p->fd>INT_MAX || p->mapoffset!=0 ||
               p->maxsize<2304 || p->chunk->offset!=0 || p->chunk->size!=2304 ||
               p->chunk->stride!=96 || p->chunk->flags!=0) {
                ++state.counts[3];
            } else {
                *available={stream,buffer,p,*p,*p->chunk};
                p->data=nullptr;
                ++state.counts[2];
            }
        }
    }
    errno=saved_errno;
    return buffer;
} catch(...) {
    std::abort(); // test fixture exceptions never cross the native C boundary
}
extern "C" MFS_PW_MEMFD_EXPORT int pw_stream_queue_buffer(pw_stream* stream,pw_buffer* buffer) try {
    static const auto native=next<decltype(&pw_stream_queue_buffer)>("pw_stream_queue_buffer");
    const int saved_errno=errno;
    {
        std::lock_guard lock(state.mutex);
        if(state.armed) {
            bool found=false;
            for(auto& lease:state.leases) {
                if(lease.buffer!=buffer) continue;
                if(lease.stream!=stream) ++state.counts[3];
                restore(lease);
                found=true;
                break;
            }
            if(!found) ++state.counts[3];
        }
    }
    errno=saved_errno;
    const int result=native(stream,buffer);
    const int result_errno=errno;
    if(result<0) {
        std::lock_guard lock(state.mutex);
        if(state.armed) ++state.counts[3];
    }
    errno=result_errno;
    return result;
} catch(...) {
    std::abort();
}
extern "C" MFS_PW_MEMFD_EXPORT void pw_stream_destroy(pw_stream* stream) try {
    static const auto native=next<decltype(&pw_stream_destroy)>("pw_stream_destroy");
    const int saved_errno=errno;
    {
        std::lock_guard lock(state.mutex);
        for(auto& lease:state.leases) {
            if(lease.stream!=stream) continue;
            ++state.counts[3]; // restoration on abnormal teardown cannot pass
            restore(lease);
        }
    }
    errno=saved_errno;
    native(stream);
} catch(...) {
    std::abort();
}
extern "C" MFS_PW_MEMFD_EXPORT int pw_stream_disconnect(pw_stream* stream) try {
    static const auto native=next<decltype(&pw_stream_disconnect)>("pw_stream_disconnect");
    const int saved_errno=errno;
    {
        std::lock_guard lock(state.mutex);
        for(auto& lease:state.leases) {
            if(lease.stream!=stream) continue;
            ++state.counts[3];
            restore(lease); // restore before disconnect can invalidate SPA data
        }
    }
    errno=saved_errno;
    return native(stream);
} catch(...) {
    std::abort();
}
