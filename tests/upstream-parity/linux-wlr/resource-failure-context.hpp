// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Separate from the unchanged mmap probe. Limits apply inside the actual libc
// boundary after UUID construction; no syscall arguments/results are invented.
void resource_failure_context(const std::filesystem::path& socket,const std::filesystem::path& directory,ResourceFault kind)
{
    std::ofstream rows(directory/"source-resource.tsv");require(rows.good(),"open resource table");
    const auto baseline=count("/proc/self/fd");const auto baseline_tasks=tasks();
    std::vector<cv::Mat> held;held.reserve(20);
    const int expected=kind==ResourceFault::Memfd ? EMFILE : EFBIG;
    for(unsigned cycle=0;cycle<20;++cycle) {
        size_t connected=0,failed=0,recovered=0;uint64_t inode=0;
        ResourceObservation observed;
        {
            auto client=std::make_shared<MfsOriginalWlr::WaylandClient>();
            require(client->connect(socket),"resource client connect failed");
            MfsOriginalWlr::WlrScreencap capture(client);
            cv::Mat image(5,7,CV_8UC3,cv::Scalar(201,73,17));const auto* owner=image.data;
            connected=count("/proc/self/fd");require(allocation_files().empty(),"preexisting resource memfd");
            sigset_t caller_mask{},return_mask{};
            struct sigaction caller_action{},return_action{};
            require(pthread_sigmask(SIG_SETMASK,nullptr,&caller_mask)==0 && sigaction(SIGXFSZ,nullptr,&caller_action)==0,
                "read caller signal context");
            resource_observation={};
            struct Arm {
                explicit Arm(ResourceFault mode) { resource_fault=mode; }
                ~Arm() { resource_fault=ResourceFault::None; }
            };
            bool success=true;
            { Arm arm(kind);success=capture.screencap(image); }
            observed=resource_observation;
            require(pthread_sigmask(SIG_SETMASK,nullptr,&return_mask)==0 && sigaction(SIGXFSZ,nullptr,&return_action)==0,
                "read return signal context");
            observed.mask_restored=observed.mask_restored && resource_same_mask(caller_mask,return_mask);
            observed.action_same=observed.action_same && resource_same_action(caller_action,return_action);
            // Check the exact truncated descriptor before any directory/file
            // observations can recycle its number. memfd failure created none.
            errno=0;
            const bool closed=observed.fd<0 || (fcntl(observed.fd,F_GETFD)<0 && errno==EBADF);
            failed=count("/proc/self/fd");const auto files=allocation_files();
            std::ofstream attempt(directory/("attempt-"+std::to_string(cycle)+".source.tsv"));
            attempt<<observed.calls<<"\t1\t"<<observed.error<<'\t'
                <<(observed.fd<0 ? UINT64_MAX : static_cast<uint64_t>(observed.fd))<<'\t'<<closed<<'\t'
                <<observed.mask_before<<'\t'<<observed.mask_restored<<'\t'<<observed.pending<<'\t'
                <<observed.consumed<<'\t'<<observed.action_same<<'\n';
            attempt.close();require(attempt.good(),"save actual resource attempt");
            require(!success && image.data==owner && observed.calls==1 && observed.error==expected
                && closed && failed==connected && files.empty(),"failed resource owner/descriptor differs");
            require(kind!=ResourceFault::Memfd || observed.before.rlim_cur>connected,"original fd limit already exhausted");
            {
                std::ofstream file(directory/("failed-"+std::to_string(cycle)+".source.bgr"),std::ios::binary);
                for(int y=0;y<5;++y) { for(int x=0;x<7;++x) {
                    require(image.at<cv::Vec3b>(y,x)==cv::Vec3b(201,73,17),"resource failure replaced pixels");
                } file.write(reinterpret_cast<const char*>(image.ptr(y)),21); }
                file.close();require(file.good(),"save resource sentinel");
            }
            require(capture.screencap(image),"resource recovery capture failed");
            recovered=count("/proc/self/fd");const auto current=allocation_files();
            require(current.size()==1 && current[0].size==140 && recovered==connected+1,"resource recovery owner differs");
            inode=current[0].inode;
            for(unsigned y=0;y<5;++y) { for(unsigned x=0;x<7;++x) { for(unsigned c=0;c<3;++c) {
                require(image.at<cv::Vec3b>(y,x)[c]==mfs_protocol_color(x,y,2-c,0),"resource recovery pixels differ");
            } } }
            held.push_back(std::move(image));
        }
        const auto final=count("/proc/self/fd");
        require(final==baseline && tasks()==baseline_tasks && allocation_files().empty(),"resource final owners differ");
        rows<<cycle<<'\t'<<baseline<<'\t'<<connected<<'\t'<<failed<<'\t'<<recovered<<'\t'<<final<<'\t'<<expected
            <<'\t'<<observed.before.rlim_cur<<'\t'<<observed.before.rlim_max<<'\t'<<observed.after.rlim_cur
            <<'\t'<<observed.after.rlim_max<<"\t0\t0\t140\t"<<inode<<"\t1\t0\t"<<observed.mask_before
            <<'\t'<<observed.mask_restored<<'\t'<<observed.pending<<'\t'<<observed.consumed<<'\t'<<observed.action_same<<'\n';
        rows.flush();require(rows.good(),"save complete resource row");
    }
    for(size_t index=0;index<held.size();++index) {
        const auto& image=held[index];
        std::ofstream file(directory/("held-"+std::to_string(index)+".source.bgr"),std::ios::binary);
        for(int y=0;y<5;++y) { file.write(reinterpret_cast<const char*>(image.ptr(y)),21); }
        file.close();require(file.good(),"save resource held image");
    }
    rows.close();require(rows.good(),"close resource table");
    std::cout<<"original Wlr resource kind="<<(kind==ResourceFault::Memfd ? "memfd" : "ftruncate")
        <<" cycles=20 kernel_failures=20 failed_fds_retained=0 recovery=20 held=20 limits_signals_restored=20 passed\n";
}
