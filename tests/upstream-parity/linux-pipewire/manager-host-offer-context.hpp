// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Complete original manager/PW bodies; only host input construction throws.
// Native socket ownership control, without daemon/Portal or successful capture.
void manager_host_offer_context(const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    { Source warm(-1,1); }
    pw_deinit(); // Fixture balance after source destruction, not product cleanup.
    std::ofstream rows(directory/"source-cases.tsv");
    require(rows.good(),"cannot save source host offer rows");
    const auto baseline_fds=count_entries("/proc/self/fd");
    const auto baseline=task_ids();
    const auto save_tasks=[](const auto& path,const auto& ids) {
        std::ofstream output(path);for(const auto& id:ids) {output<<id<<'\n';}
        output.close();require(output.good(),"cannot save host offer task identities");
    };
    save_tasks(directory/"baseline-task-ids.txt",baseline);
    for(unsigned cycle=0;cycle<20;++cycle) {
        uint64_t before=0,after_error=0,after_drop=0;
        unsigned attempts=0;
        bool refused=false,preserved=false;
        {
            std::array<int,2> sockets{-1,-1};
            require(::socketpair(AF_UNIX,SOCK_STREAM|SOCK_CLOEXEC,0,sockets.data())==0,"cannot create real host offer socketpair");
            ProbeSocket helper{sockets[0]},peer{sockets[1]};
            const auto original_cookie=socket_cookie(helper.fd);
            require(original_cookie!=0,"host socket identity is zero");
            ProbeSocket offer{::dup(helper.fd)};
            require(offer.fd>=0 && offer.fd!=helper.fd,"cannot duplicate live host-owned socket");
            offer.cookie=original_cookie;offer.offered=true;
            const int helper_fd=std::exchange(helper.fd,-1);
            require(::close(helper_fd)==0,"cannot end original host helper socket");
            errno=0;require(::fcntl(helper_fd,F_GETFD)==-1 && errno==EBADF,"helper socket remained open");
            before=socket_cookie(offer.fd);
            require(before==original_cookie,"helper destruction changed offered socket identity");
            MfsOriginalPipeWire::host_input={};
            auto& state=MfsOriginalPipeWire::host_input;
            state.enabled=true;state.outcome=3;state.capture_fd=offer.fd;
            {
                struct Balance {
                    ~Balance() {if(MfsOriginalPipeWire::host_input.native_constructed) {pw_deinit();}}
                } balance; // Native manager destruction must precede deinit.
                MfsOriginalPipeWire::LinuxControlUnitConfig config;
                config.screencap_method=MaaLinuxScreencapMethod_PipeWire;
                config.input_method=MaaLinuxInputMethod_UInput;
                config.pw_socket_fd=offer.fd;config.pw_node_id=0;
                config.uinput_path="/capture-only-host";config.uinput_screen_width=17;config.uinput_screen_height=19;
                MfsOriginalPipeWire::LinuxControlUnitMgr manager(config);
                bool threw=false;
                try {static_cast<void>(manager.connect());}
                catch(const MfsOriginalPipeWire::HostInputCreateFailure&) {threw=true;}
                require(threw && !manager.connected() && manager.screencap_ && !manager.input_,"source host construction error lost retained native owner");
                after_error=socket_cookie(offer.fd);
                cv::Mat output(1,1,CV_8UC3,cv::Scalar(7,9,11));
                const auto* data=output.data;const int owners=output.u->refcount;
                refused=!manager.screencap(output);
                preserved=output.data==data && output.u->refcount==owners && output.at<cv::Vec3b>(0,0)==cv::Vec3b(7,9,11);
                require(manager.inactive() && manager.get_features()==0,"failed host created an input/features owner");
                require(state.created==0 && state.initialized==0 && state.inactive==0 && state.destroyed==0,"failed host constructor was treated as a live input");
                attempts=state.creation_attempts;
            }
            after_drop=socket_cookie(offer.fd);
            require(before==after_error && before==after_drop,"native pre-init teardown consumed/replaced the unconsumed socket");
            // The product's residual is observed ABOVE. Only the fixture closes
            // it now, after complete native teardown and the same-cookie proof.
            const int reclaim=std::exchange(offer.fd,-1);
            require(::close(reclaim)==0,"cannot reclaim the same unconsumed fixture socket");
            state.enabled=false;
        }
        const auto final_fds=count_entries("/proc/self/fd");
        const auto final_tasks=task_ids();
        save_tasks(directory/("closed-"+std::to_string(cycle)+"-task-ids.txt"),final_tasks);
        rows<<cycle<<"\t98\t"<<attempts<<"\t0\t0\t0\t1\t"<<before<<'\t'<<after_error<<'\t'<<after_drop
            <<'\t'<<refused<<'\t'<<preserved<<"\t1\t1\t"<<baseline_fds<<'\t'<<final_fds<<'\t'<<baseline.size()<<'\t'<<final_tasks.size()<<"\t1\n";
        rows.flush();require(rows.good(),"source host offer row write failed");
        require(attempts==1 && refused && preserved && final_fds==baseline_fds && final_tasks==baseline,"source pre-init host error resources/output differ");
    }
    rows.close();require(rows.good(),"source host offer records close failed");
    std::cout<<"original PipeWire host offer cycles=20 create_errors=20 native_init=0 retained_identity=20 unconsumed_after_destroy=20 fixture_reclaims=20 capture=0 passed\n";
}
