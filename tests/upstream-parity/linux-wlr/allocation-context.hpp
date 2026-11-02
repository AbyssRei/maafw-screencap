// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Native kernel resource limit in the owned test process, never an SDK policy.
struct AllocationLimit {
    rlimit before{};
    uint64_t vm=0,selected=0;
    bool active=false;
    AllocationLimit() {
        uint64_t pages=0;
        { std::ifstream file("/proc/self/statm");require(bool(file>>pages),"read allocation VmSize"); }
        const long page=sysconf(_SC_PAGESIZE);
        require(page>0 && pages<=UINT64_MAX/static_cast<uint64_t>(page),"allocation VmSize overflow");
        vm=pages*static_cast<uint64_t>(page);
        require(vm<=UINT64_MAX-33554432,"allocation limit overflow");selected=vm+33554432;
        require(getrlimit(RLIMIT_AS,&before)==0 && (before.rlim_cur==RLIM_INFINITY || selected<=before.rlim_cur),
            "cannot lower owned address-space limit");
        rlimit limit=before;limit.rlim_cur=selected;
        require(setrlimit(RLIMIT_AS,&limit)==0,"set owned address-space limit");active=true;
        allocation_observing=true;
    }
    void restore() {
        allocation_observing=false;
        require(setrlimit(RLIMIT_AS,&before)==0,"restore original address-space limit");active=false;
        rlimit actual{};
        require(getrlimit(RLIMIT_AS,&actual)==0 && actual.rlim_cur==before.rlim_cur
            && actual.rlim_max==before.rlim_max,"original address-space limit identity differs");
    }
    ~AllocationLimit() {
        allocation_observing=false;
        if(active && setrlimit(RLIMIT_AS,&before)!=0) { std::terminate(); }
    }
};
struct AllocationFile { int fd;uint64_t inode,size; };
std::vector<AllocationFile> allocation_files()
{
    std::vector<AllocationFile> result;
    for(const auto& entry:std::filesystem::directory_iterator("/proc/self/fd")) {
        std::error_code error;
        const auto target=std::filesystem::read_symlink(entry.path(),error).string();
        if(error || target.find("/memfd:maafw-wl-buffer-")!=0) { continue; }
        struct stat value{};const int fd=std::stoi(entry.path().filename().string());
        require(fstat(fd,&value)==0 && value.st_size>=0,"stat actual Wlr memfd");
        result.push_back({fd,static_cast<uint64_t>(value.st_ino),static_cast<uint64_t>(value.st_size)});
    }
    return result;
}
void allocation_context(const std::filesystem::path& socket,const std::filesystem::path& directory)
{
    std::ofstream rows(directory/"source-allocation.tsv");require(rows.good(),"open allocation table");
    const auto baseline=count("/proc/self/fd");const auto baseline_tasks=tasks();
    std::vector<cv::Mat> held;held.reserve(20);
    for(unsigned cycle=0;cycle<20;++cycle) {
        size_t connected=0,failed=0,recovered=0;uint64_t vm=0,selected=0,old_limit=0,failed_inode=0,recovered_inode=0;
        int failed_fd=-1;
        {
            auto client=std::make_shared<MfsOriginalWlr::WaylandClient>();
            require(client->connect(socket),"allocation client connect failed");
            MfsOriginalWlr::WlrScreencap capture(client);
            cv::Mat output(5,7,CV_8UC3,cv::Scalar(201,73,17));const auto* owner=output.data;
            connected=count("/proc/self/fd");require(allocation_files().empty(),"unexpected preexisting Wlr memfd");
            allocation_map_calls=0;allocation_map_errno=0;allocation_map_fd=-1;allocation_map_failed=false;
            bool result=true;
            {
                AllocationLimit limit;vm=limit.vm;selected=limit.selected;old_limit=limit.before.rlim_cur;
                result=capture.screencap(output);limit.restore();
            }
            failed=count("/proc/self/fd");const auto files=allocation_files();
            std::ofstream attempt(directory/("attempt-"+std::to_string(cycle)+".source.tsv"));
            attempt<<vm<<'\t'<<selected<<'\t'<<old_limit<<'\t'<<allocation_map_calls<<'\t'
                <<allocation_map_errno<<'\t'<<allocation_map_fd<<'\t'<<connected<<'\t'<<failed<<'\t'<<files.size()
                <<'\t'<<allocation_map_failed<<'\n';
            attempt.close();require(attempt.good(),"save source allocation attempt");
            require(!result && output.data==owner && allocation_map_calls==1 && allocation_map_failed
                && allocation_map_errno==ENOMEM,"native failed mmap or caller ownership differs");
            require(files.size()==1 && files[0].fd==allocation_map_fd && files[0].size==268435456
                && failed==connected+1,"source failed mmap did not retain its actual fd");
            failed_fd=files[0].fd;failed_inode=files[0].inode;
            for(int y=0;y<5;++y) { for(int x=0;x<7;++x) {
                require(output.at<cv::Vec3b>(y,x)==cv::Vec3b(201,73,17),"failed allocation replaced pixels");
            } }
            {
                std::ofstream file(directory/("failed-"+std::to_string(cycle)+".source.bgr"),std::ios::binary);
                for(int y=0;y<5;++y) { file.write(reinterpret_cast<const char*>(output.ptr(y)),21); }
                file.close();require(file.good(),"save failed allocation pixels");
            }
            require(capture.screencap(output),"allocation recovery capture failed");
            recovered=count("/proc/self/fd");const auto current=allocation_files();
            require(current.size()==1 && current[0].size==140 && current[0].inode!=failed_inode
                && recovered==connected+1,"source failed buffer replacement lifetime differs");
            recovered_inode=current[0].inode;
            for(unsigned y=0;y<5;++y) { for(unsigned x=0;x<7;++x) { for(unsigned c=0;c<3;++c) {
                require(output.at<cv::Vec3b>(y,x)[c]==mfs_protocol_color(x,y,2-c,0),"allocation recovery pixel mismatch");
            } } }
            held.push_back(std::move(output));
        }
        const auto final=count("/proc/self/fd");
        require(final==baseline && tasks()==baseline_tasks && allocation_files().empty(),"allocation final resources differ");
        // The rows file itself is already in baseline for this measurement.
        rows<<cycle<<'\t'<<baseline<<'\t'<<connected<<'\t'<<failed<<'\t'<<recovered<<'\t'<<final
            <<'\t'<<vm<<'\t'<<selected<<'\t'<<old_limit<<'\t'<<old_limit<<"\t268435456\t12\t"<<failed_fd
            <<'\t'<<failed_inode<<'\t'<<recovered_inode<<"\t0\t1\t1\n";
        rows.flush();require(rows.good(),"save complete allocation row");
    }
    for(size_t index=0;index<held.size();++index) {
        const auto& image=held[index];
        std::ofstream file(directory/("held-"+std::to_string(index)+".source.bgr"),std::ios::binary);
        for(int y=0;y<5;++y) { file.write(reinterpret_cast<const char*>(image.ptr(y)),21); }
        file.close();require(file.good(),"save held allocation recovery pixels");
    }
    rows.close();require(rows.good(),"close allocation table");
    std::cout<<"original Wlr allocation cycles=20 failed_mmap=20 fd_retained=20 recovery=20 held=20 limits_restored=20 passed\n";
}
