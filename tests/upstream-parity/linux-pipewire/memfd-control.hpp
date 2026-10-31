// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included after source-context's require(). No native capture code is edited.
#include "memfd-probe.h"
struct MemFdControl {
    template<typename F> static F symbol(const char* name) {
        void* pointer=dlsym(RTLD_DEFAULT,name);
        const char* expected=std::getenv("MFS_P1_PIPEWIRE_MEMFD_PROBE_LIBRARY");
        Dl_info info{};
        require(pointer && expected && dladdr(pointer,&info)!=0 && info.dli_fname,
            "explicit MemFd intervention is absent");
        require(std::filesystem::canonical(info.dli_fname)==std::filesystem::canonical(expected),
            "MemFd intervention loaded from another library");
        return reinterpret_cast<F>(pointer);
    }
    decltype(&mfs_pw_memfd_probe_arm) arm=symbol<decltype(arm)>("mfs_pw_memfd_probe_arm");
    decltype(&mfs_pw_memfd_probe_read) read=symbol<decltype(read)>("mfs_pw_memfd_probe_read");
    decltype(&mfs_pw_memfd_probe_disarm) disarm=symbol<decltype(disarm)>("mfs_pw_memfd_probe_disarm");
    void begin() const { require(arm()==0,"MemFd probe arm failed"); }
    void finish(int cycle,const std::filesystem::path& root) const {
        std::array<uint64_t,4> counts{};
        for(uint32_t i=0;i<counts.size();++i) counts[i]=read(i);
        std::ofstream file(root/("probe-"+std::to_string(cycle)+".tsv"));
        file<<"cycle\tdequeued\trestored\tlive\terrors\tcopies\town_maps\tunmaps\n"
            <<cycle<<'\t'<<counts[0]<<'\t'<<counts[1]<<'\t'<<counts[2]<<'\t'<<counts[3]
            <<"\tunknown\tunknown\tunknown\n";
        file.close();
        require(file.good(),"MemFd source record write failed");
        require(disarm()==0 && counts[0]>=3 && counts[0]==counts[1] && counts[2]==0 && counts[3]==0,
            "MemFd real dequeue/restore/layout/FD checks failed");
    }
};
