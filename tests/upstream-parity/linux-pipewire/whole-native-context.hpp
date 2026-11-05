// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included inside the existing caller's anonymous namespace. Same real source,
// producer, conversion and task/FD observations; complete original C factories
// and actual virtual calls replace the earlier extracted-body manager caller.
void whole_manager_context(const char* socket,uint32_t node,const std::filesystem::path& directory,bool supplied_fd) {
    std::filesystem::create_directories(directory);
    require(std::string_view(MaaLinuxControlUnitGetVersion())=="mfs-p1-whole-pipewire","wrong original whole API version");
    std::ofstream origin(directory/"whole-origin.txt");
    origin<<"whole_original_manager=1 full_original_api=1 real_pipewire=1 real_NoneInput=1 input_actions=0\n";
    origin.close();require(origin.good(),"cannot save whole original origin");
    std::ofstream cases(directory/"source-cases.tsv"),owners(directory/"source-owners.tsv");
    require(cases.good() && owners.good(),"cannot save whole source records");
    cv::Mat warm(HEIGHT,WIDTH,CV_8UC3,cv::Scalar(1,2,3)),converted;cv::cvtColor(warm,converted,cv::COLOR_RGB2BGR);
    const size_t base_fds=count_entries("/proc/self/fd");const auto baseline=task_ids();
    task_snapshot(directory,"baseline",baseline);
    const auto save_tasks=[&](const std::filesystem::path& path,const auto& ids) {
        std::ofstream out(path);for(const auto& id:ids){out<<id<<'\n';}out.close();require(out.good(),"cannot save whole manager task identities");
    };
    save_tasks(directory/"baseline-task-ids.txt",baseline);
    const int sessions=supplied_fd?1:2;
    std::vector<HeldImage> held;held.reserve(static_cast<size_t>(20*sessions*3));
    for(int cycle=0;cycle<20;++cycle) {
        const int fd=supplied_fd?connect_socket(socket):-1;
        const auto config=std::string("{\"screencap_method\":4,\"input_method\":0,\"pw_socket_fd\":")+
            std::to_string(fd)+",\"pw_node_id\":"+std::to_string(node)+'}';
        std::vector<Task> live;
        struct Balance {int initialized=0;~Balance(){for(int i=0;i<initialized;++i){pw_deinit();}}} balance;
        {
            std::unique_ptr<MAA_CTRL_UNIT_NS::LinuxControlUnitAPI,decltype(&MaaLinuxControlUnitDestroy)> unit(
                MaaLinuxControlUnitCreate(config.c_str()),MaaLinuxControlUnitDestroy);
            require(bool(unit),"whole original factory rejected explicit PipeWire configuration");
            const auto cold=MfsWholeLinux::observe(*unit);
            const auto observe=[&](int session,int stage,const MfsWholeLinux::Owners* expected=nullptr) {
                const auto state=MfsWholeLinux::observe(*unit);
                const uint64_t features=unit->get_features();
                const bool connected=unit->connected();
                const bool actual=state.capture && MfsWholeLinux::pipewire_type(*unit);
                owners<<cycle<<'\t'<<session<<'\t'<<stage<<'\t'<<state.manager<<'\t'<<state.capture<<'\t'<<state.input
                    <<'\t'<<(actual?4:0)<<'\t'<<state.input_none<<'\t'<<features<<'\t'<<connected<<'\n';owners.flush();
                require(owners.good() && state.manager==cold.manager,"whole original owner record/manager differs");
                if(stage==0) {require(!state.capture && !state.input && !features && !connected,"cold whole manager already has native owners");}
                else {require(actual && state.input && state.input_none==1 && features==3 && connected,"whole original native/input type or features differs");}
                if(expected) {require(state==*expected,"whole original capture/inactive changed owners");}
                return state;
            };
            observe(-1,0);
            std::string uuid;require(unit->request_uuid(uuid) && uuid==std::to_string(fd)+"_"+std::to_string(node)+"_-1","whole original UUID differs");
            const auto info=json::value(unit->get_info()).to_string();
            const auto save_text=[&](const std::filesystem::path& p,std::string_view bytes) {
                std::ofstream out(p,std::ios::binary);out.write(bytes.data(),static_cast<std::streamsize>(bytes.size()));
                out.close();require(out.good(),"whole manager metadata write failed");
            };
            save_text(directory/("api-"+std::to_string(cycle)+".uuid"),uuid);
            save_text(directory/("api-"+std::to_string(cycle)+".info"),info);
            cv::Mat output;require(!unit->screencap(output) && output.empty(),"cold whole manager changed output");
            for(int session=0;session<sessions;++session) {
                const bool connected=unit->connect();++balance.initialized;
                require(connected && unit->connected(),"whole original PipeWire manager connect failed");
                const auto active=observe(session,1);
                for(int call=0;call<3;++call) {
                    if(call==1) {require(unit->inactive(),"whole original inactive failed");observe(session,3,&active);}
                    require(unit->screencap(output),"whole original virtual PipeWire capture failed");
                    const auto stem="manager-"+std::to_string(cycle)+"-session-"+std::to_string(session)+"-call-"+std::to_string(call);
                    save(output,directory/(stem+".bgr"));const auto serial=known_pixels(output);
                    observe(session,call==0?2:call==1?4:5,&active);
                    held.push_back({std::move(output),serial});
                    for(const auto& image:held) {require(known_pixels(image.image)==image.marker,"whole original changed held pixels");}
                }
                require(unit->request_uuid(uuid) && uuid==std::to_string(fd)+"_"+std::to_string(node)+"_-1"
                    && json::value(unit->get_info()).to_string()==info,"whole original capture/inactive changed metadata");
            }
            live=live_tasks();
        } // Real virtual manager/backend/input destruction, then fixture deinit.
        for(int i=0;i<balance.initialized;++i) {pw_deinit();}balance.initialized=0;
        bool consumed=false;
        if(supplied_fd) {errno=0;consumed=fcntl(fd,F_GETFD)==-1 && errno==EBADF;require(consumed,"whole original did not close consumed socket");}
        const auto fds=count_entries("/proc/self/fd");const auto immediate=task_ids();bool settled=immediate==baseline;
        if(fds==base_fds && !settled && only_retiring_tasks(immediate,baseline,live)) {
            task_snapshot(directory,"cycle-"+std::to_string(cycle)+"-retiring",immediate);
            const auto start=std::chrono::steady_clock::now();
            for(;;) {
                const auto current=task_ids();if(current==baseline){settled=true;break;}
                if(!only_retiring_tasks(current,baseline,live) || std::chrono::steady_clock::now()-start>=std::chrono::milliseconds(100)){break;}
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
        }
        const auto closed=task_ids();task_snapshot(directory,"cycle-"+std::to_string(cycle)+"-closed",closed);
        save_tasks(directory/("closed-"+std::to_string(cycle)+"-task-ids.txt"),closed);
        cases<<cycle<<'\t'<<fd<<'\t'<<node<<'\t'<<sessions<<"\t0\t3\t"<<base_fds<<'\t'<<fds<<'\t'<<baseline.size()
            <<'\t'<<(closed==baseline)<<'\t'<<consumed<<'\t'<<sessions*3<<'\n';cases.flush();
        require(cases.good() && fds==base_fds && settled && closed==baseline,"whole source native resources differ");
    }
    for(size_t i=0;i<held.size();++i) {
        require(known_pixels(held[i].image)==held[i].marker,"whole original held image changed after managers ended");
        save(held[i].image,directory/("held-"+std::to_string(i)+".bgr"));
    }
    std::cout<<"original whole PipeWire manager mode="<<(supplied_fd?"fd":"daemon")<<" objects=20 sessions="<<20*sessions
        <<" captures="<<held.size()<<" held="<<held.size()<<" owner_observations="<<20*(1+5*sessions)
        <<" original_class_api=1 real_NoneInput=1 active_features=3 resources_balanced=1 input_actions=0 full_P1_acceptance=0 passed\n";
}
