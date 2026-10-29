// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Explicit native manager acceptance fixture, included inside the test namespace.
// Product calls use the exact extracted manager bodies and real PipeWire source.
void manager_context(const char* socket,uint32_t node,const std::filesystem::path& artifacts,bool supplied_fd) {
    std::filesystem::create_directories(artifacts);
    cv::Mat warm(HEIGHT,WIDTH,CV_8UC3,cv::Scalar(1,2,3)),converted;
    cv::cvtColor(warm,converted,cv::COLOR_RGB2BGR);
    const size_t base_fds=count_entries("/proc/self/fd");
    const auto baseline=task_ids();
    std::vector<HeldImage> held;
    const int sessions=supplied_fd?1:2;
    held.reserve(static_cast<size_t>(20*sessions*3));
    for(int cycle=0;cycle<20;++cycle) {
        int fd=-1;
        const auto live_before=live_tasks();
        std::vector<Task> live;
        {
            MfsOriginalPipeWire::LinuxControlUnitConfig config;
            config.screencap_method=MaaLinuxScreencapMethod_PipeWire;config.pw_node_id=node;
            if(supplied_fd) {fd=connect_socket(socket);config.pw_socket_fd=fd;}
            // Balance source constructor pw_init only AFTER the manager and all
            // backend native resources have ended. No product deinit is added.
            struct Balance {
                int initialized=0;
                ~Balance() {for(int i=0;i<initialized;++i) {pw_deinit();}}
            } balance;
            MfsOriginalPipeWire::LinuxControlUnitMgr manager(config);
            cv::Mat output;
            require(!manager.screencap(output) && output.empty(),"unconnected manager changed output");
            std::string uuid;
            require(manager.request_uuid(uuid) && uuid==std::to_string(fd)+"_"+std::to_string(node)+"_-1",
                "native source manager UUID differs from configuration");
            for(int session=0;session<sessions;++session) {
                const auto connected=manager.connect();
                ++balance.initialized;
                require(connected && manager.connected(),"original PipeWire manager connect failed");
                for(int call=0;call<3;++call) {
                    if(call==1) {require(manager.inactive(),"original manager inactive failed");}
                    require(manager.screencap(output),"original PipeWire manager forwarding failed");
                    save(output,artifacts/("manager-"+std::to_string(cycle)+"-session-"+std::to_string(session)+"-call-"+std::to_string(call)+".bgr"));
                    const auto serial=known_pixels(output);
                    held.push_back({output,serial});
                    for(const auto& image:held) {require(known_pixels(image.image)==image.marker,"manager changed held source pixels");}
                }
            }
            live=live_tasks();
        }
        if(supplied_fd) {
            errno=0;
            require(fcntl(fd,F_GETFD)==-1 && errno==EBADF,"manager did not close consumed socket");
        }
        const size_t fds=count_entries("/proc/self/fd");
        const auto immediate=task_ids();
        bool settled=immediate==baseline;
        if(fds==base_fds && !settled && only_retiring_tasks(immediate,baseline,live)) {
            task_snapshot(artifacts,"manager-"+std::to_string(cycle)+"-kernel-retirement",immediate);
            const auto start=std::chrono::steady_clock::now();
            for(;;) {
                const auto current=task_ids();
                if(current==baseline) {settled=true;break;}
                if(!only_retiring_tasks(current,baseline,live) ||
                    std::chrono::steady_clock::now()-start>=std::chrono::milliseconds(100)) {break;}
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
        }
        if(fds!=base_fds || !settled) {task_snapshot(artifacts,"manager-"+std::to_string(cycle)+"-mismatch",immediate);}
        std::cout<<"source manager cycle="<<cycle<<" fds="<<fds<<" baseline_fds="<<base_fds
            <<" immediate_tasks="<<immediate.size()<<" settled="<<settled<<" held="<<held.size()<<'\n';
        require(fds==base_fds && settled,"source manager resource mismatch or unretired/unknown/live task");
        require(live_before.size()==baseline.size(),"manager initial task premise changed");
    }
    std::exception_ptr error;
    std::thread reader([&] {
        try {for(const auto& image:held) {require(known_pixels(image.image)==image.marker,"post-manager source image changed");}}
        catch(...) {error=std::current_exception();}
    });
    reader.join();if(error) {std::rethrow_exception(error);}
    std::cout<<"original PipeWire manager mode="<<(supplied_fd?"fd":"daemon")
        <<" objects=20 sessions="<<20*sessions<<" captures="<<20*sessions*3
        <<" retained="<<held.size()<<" consumed_sockets_closed="<<(supplied_fd?20:0)<<" resources_balanced=1 passed\n";
}
