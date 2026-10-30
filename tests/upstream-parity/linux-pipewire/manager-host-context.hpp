// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Real PW manager/captures; explicit host init outcomes and owned socketpairs.
// No original Portal authorization, device input or full compat-API claim.
void manager_host_context(const char* socket,uint32_t node,const std::filesystem::path& artifacts) {
    std::filesystem::create_directories(artifacts);
    std::ofstream rows(artifacts/"source-cases.tsv",std::ios::out|std::ios::trunc);
    require(static_cast<bool>(rows),"cannot create source host records");
    cv::Mat warm(HEIGHT,WIDTH,CV_8UC3,cv::Scalar(1,2,3)),converted;
    cv::cvtColor(warm,converted,cv::COLOR_RGB2BGR);
    const auto base_fds=count_entries("/proc/self/fd");
    const auto baseline=task_ids();
    const auto save_tasks=[](const std::filesystem::path& path,const auto& ids) {
        std::ofstream output(path);for(const auto& id:ids) {output<<id<<'\n';}output.close();
        require(static_cast<bool>(output),"cannot save actual host task identities");
    };
    save_tasks(artifacts/"baseline-task-ids.txt",baseline);
    std::vector<HeldImage> held;held.reserve(60);
    unsigned connected_count=0,false_count=0,error_count=0;
    for(unsigned cycle=0;cycle<20;++cycle) {
        const unsigned outcome=cycle%3;
        // A controlled host hands off ONE live private socket, then its helper
        // owner ends before native construction. This is not a DBus Portal.
        int fd=-1;
        bool helper_ended=false;
        {
            struct Offer {
                int fd;
                bool* ended;
                ~Offer() { if(fd>=0) { ::close(fd); }*ended=true; }
            } helper{connect_socket(socket),&helper_ended};
            fd=std::exchange(helper.fd,-1);
        }
        const bool offer_open=::fcntl(fd,F_GETFD)>=0;
        require(helper_ended && offer_open,"host helper destruction closed the offered socket");
        MfsOriginalPipeWire::host_input={};
        auto& state=MfsOriginalPipeWire::host_input;
        state.enabled=true;state.outcome=outcome;state.capture_fd=fd;
        std::vector<Task> live;
        int result=-1,error_code=0;
        uint64_t features_after=0;
        {
            struct Balance {
                ~Balance() { if(MfsOriginalPipeWire::host_input.native_constructed) { pw_deinit(); } }
            } balance; // Always after the manager and host owner.
            MfsOriginalPipeWire::LinuxControlUnitConfig config;
            config.screencap_method=MaaLinuxScreencapMethod_PipeWire;config.input_method=MaaLinuxInputMethod_UInput;
            config.pw_socket_fd=fd;config.pw_node_id=node;config.uinput_path="/capture-only-host";
            config.uinput_screen_width=17;config.uinput_screen_height=19;config.use_win32_vk_code=true;
            MfsOriginalPipeWire::LinuxControlUnitMgr manager(config);
            require(!manager.connected() && manager.get_features()==0,"new host manager state differs");
            std::string uuid;
            require(manager.request_uuid(uuid) && uuid==std::to_string(fd)+"_"+std::to_string(node)+"_/capture-only-host","source host UUID lost input identity");
            cv::Mat output;
            require(!manager.screencap(output) && output.empty(),"unconnected host manager changed output");
            result=2;
            try { result=manager.connect()?1:0; }
            catch(const MfsOriginalPipeWire::HostInputInitFailure&) { error_code=97;require(outcome==2,"unexpected host init exception"); }
            const int expected=outcome==0?1:outcome==1?0:2;
            require(result==expected && manager.connected()==(outcome==0),"source host init result/state differs");
            require(state.created==1 && state.initialized==1 && manager.get_features()==3,"source host owner/features missing");
            if(result==1) { ++connected_count; } else if(result==0) { ++false_count; } else { ++error_count; }
            for(unsigned call=0;call<3;++call) {
                if(call==1) { require(manager.inactive(),"source host inactive failed"); }
                require(manager.screencap(output),"source host failure lost real capture forwarding");
                save(output,artifacts/("host-"+std::to_string(cycle)+"-call-"+std::to_string(call)+".bgr"));
                const auto serial=known_pixels(output);
                held.push_back({std::move(output),serial});
                for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"source host changed held image"); }
            }
            features_after=manager.get_features();
            require(features_after==3,"source inactive removed host feature owner");
            live=live_tasks();
        }
        errno=0;
        const bool socket_closed=::fcntl(fd,F_GETFD)==-1 && errno==EBADF;
        const auto fds=count_entries("/proc/self/fd");
        const auto immediate=task_ids();
        bool settled=immediate==baseline;
        if(fds==base_fds && !settled && only_retiring_tasks(immediate,baseline,live)) {
            const auto start=std::chrono::steady_clock::now();
            for(;;) {
                const auto current=task_ids();
                if(current==baseline) { settled=true;break; }
                if(!only_retiring_tasks(current,baseline,live) || std::chrono::steady_clock::now()-start>=std::chrono::milliseconds(100)) { break; }
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
        }
        rows<<cycle<<'\t'<<outcome<<'\t'<<result<<'\t'<<error_code<<'\t'<<state.created<<'\t'<<state.initialized<<'\t'<<state.inactive<<'\t'<<state.destroyed
            <<'\t'<<state.native_closed_before_input<<'\t'<<state.host_fds_closed<<'\t'<<state.errors<<'\t'<<socket_closed
            <<'\t'<<base_fds<<'\t'<<fds<<'\t'<<baseline.size()<<'\t'<<settled<<'\t'<<state.capture_observation_error
            <<'\t'<<state.close_errors[0]<<'\t'<<state.close_errors[1]<<'\t'<<features_after<<'\t'<<helper_ended<<'\t'<<offer_open<<'\n';rows.flush();
        require(static_cast<bool>(rows),"source host case record failed");
        const auto closed_tasks=task_ids();
        save_tasks(artifacts/("closed-"+std::to_string(cycle)+"-task-ids.txt"),closed_tasks);
        require(socket_closed && state.created==1 && state.initialized==1 && state.inactive==1 && state.destroyed==1
            && state.native_closed_before_input==1 && state.host_fds_closed==2 && state.errors==0 && fds==base_fds && settled && closed_tasks==baseline,
            "source host/native close order/resources differ");
        state.enabled=false;
    }
    for(size_t index=0;index<held.size();++index) {
        require(known_pixels(held[index].image)==held[index].marker,"post-close source host image changed");
        save(held[index].image,artifacts/("held-"+std::to_string(index)+".source.bgr"));
    }
    require(connected_count==7 && false_count==7 && error_count==6,"host outcomes incomplete");
    std::cout<<"original PipeWire host manager objects=20 init_true=7 init_false=7 init_error=6 captures=60 retained=60 helper_ended_before_connect=20 native_closed_before_host=20 host_fds_closed=40 input_actions=0 passed\n";
}
