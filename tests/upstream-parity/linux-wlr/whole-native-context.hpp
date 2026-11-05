// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Forward every real recursive roundtrip immediately; no allocation, logging or
// exception at the native boundary. All calls use this process's sole channel.
namespace {uint64_t whole_roundtrips=0;bool whole_roundtrip_overflow=false;}
extern "C" int __real_wl_display_roundtrip(wl_display*);
extern "C" int __wrap_wl_display_roundtrip(wl_display* display) {
    if(whole_roundtrips==UINT64_MAX){whole_roundtrip_overflow=true;}else{++whole_roundtrips;}
    return __real_wl_display_roundtrip(display);
}
namespace {
int whole_context(int argc,char** argv) {
    require(argc==4,"usage: whole-native-context <private socket> <new artifacts> <actual libwayland>");
    Dl_info library{};
    require(dladdr(reinterpret_cast<void*>(&wl_display_connect),&library)!=0 && library.dli_fname
        && std::filesystem::canonical(library.dli_fname)==std::filesystem::canonical(argv[3]),"wrong whole Wlr native library");
    const std::filesystem::path directory=argv[2];require(std::filesystem::create_directory(directory),"whole Wlr records already exist");
    require(std::string_view(MaaLinuxControlUnitGetVersion())=="mfs-p1-whole-wlr","wrong whole original API version");
    text(directory/"socket-path.txt",argv[1]);
    text(directory/"whole-origin.txt","whole_original_manager=1 full_original_api=1 real_Wlr=1 real_NoneInput=1 input_actions=0\n");
    const auto configuration=[](const char* socket){return json::value(json::object{{"screencap_method",1},{"input_method",0},{"wlr_socket_path",socket}}).to_string();};
    const auto config=configuration(argv[1]);
    cv::Mat warm(48,64,CV_8UC4,cv::Scalar(0,0,255,255)),converted;cv::cvtColor(warm,converted,cv::COLOR_BGRA2BGR);
    std::ofstream cases(directory/"source-cases.tsv"),owners(directory/"source-owners.tsv");
    require(cases.good() && owners.good(),"cannot create whole Wlr records");
    const auto baseline_fds=fds();const auto baseline_tasks=tasks();task_record(directory/"baseline-task-ids.txt",baseline_tasks);
    std::vector<cv::Mat> held;held.reserve(60);
    for(unsigned cycle=0;cycle<20;++cycle) {
        std::weak_ptr<MAA_CTRL_UNIT_NS::WaylandClient> weak;
        {
            std::unique_ptr<MAA_CTRL_UNIT_NS::LinuxControlUnitAPI,decltype(&MaaLinuxControlUnitDestroy)> unit(
                MaaLinuxControlUnitCreate(config.c_str()),MaaLinuxControlUnitDestroy);
            require(bool(unit),"whole original Wlr factory failed");
            const auto cold=MfsWholeLinux::observe(*unit);
            const auto observe=[&](unsigned stage,const MfsWholeLinux::Owners* previous=nullptr,uintptr_t expected_client=0) {
                const auto state=MfsWholeLinux::observe(*unit);const auto& client=MfsWholeLinux::client(*unit);
                const auto identity=reinterpret_cast<uintptr_t>(client.get());
                const auto features=unit->get_features();const auto connected=unit->connected();
                const bool actual=state.capture && MfsWholeLinux::wlr_type(*unit);
                const bool native=client && client->connected();
                owners<<cycle<<'\t'<<stage<<'\t'<<state.manager<<'\t'<<state.capture<<'\t'<<state.input<<'\t'<<identity
                    <<'\t'<<actual<<'\t'<<state.input_none<<'\t'<<features<<'\t'<<connected<<'\t'<<whole_roundtrips<<'\t'<<native<<'\n';owners.flush();
                require(owners.good() && state.manager==cold.manager && !whole_roundtrip_overflow,"whole Wlr identity/record failure");
                if(stage==0){require(!state.capture && !state.input && !client && features==0 && !connected,"whole Wlr cold owners differ");}
                else{require(actual && state.input_none==1 && features==3 && connected && native,"whole Wlr actual native/input state differs");}
                if(previous){require(state==*previous,"whole Wlr capture/inactive changed owners");}
                if(expected_client){require(identity==expected_client,"whole Wlr reconstructed its shared client");}
                return state;
            };
            observe(0);
            std::string uuid;require(unit->request_uuid(uuid) && uuid==std::string(argv[1])+"_-1","whole Wlr UUID differs");
            const auto info=json::value(unit->get_info()).to_string();
            text(directory/("api-"+std::to_string(cycle)+".uuid"),uuid);text(directory/("api-"+std::to_string(cycle)+".info"),info);
            cv::Mat output;require(!unit->screencap(output) && output.empty(),"whole Wlr cold capture changed output");
            require(unit->connect(),"whole Wlr initial connect failed");const auto active=observe(1);
            const auto identity=reinterpret_cast<uintptr_t>(MfsWholeLinux::client(*unit).get());weak=MfsWholeLinux::client(*unit);
            for(unsigned call=0;call<3;++call) {
                if(call==1){require(unit->inactive(),"whole Wlr inactive failed");observe(3,&active,identity);}
                if(call==2){const auto before=whole_roundtrips;require(unit->connect(),"whole Wlr reconstruction failed");
                    require(whole_roundtrips==before,"whole Wlr reconstruction repeated client roundtrip");observe(5,nullptr,identity);}
                require(unit->screencap(output),"whole Wlr virtual capture failed");
                save(directory/("manager-"+std::to_string(cycle)+"-"+std::to_string(call)+".bgr"),output);
                observe(call==0?2:call==1?4:6,call<2?&active:nullptr,identity);
                held.push_back(std::move(output));for(const auto& image:held){pixels(image);}
            }
            require(unit->request_uuid(uuid) && uuid==std::string(argv[1])+"_-1" && json::value(unit->get_info()).to_string()==info,"whole Wlr metadata changed");
        }
        const auto closed=tasks();task_record(directory/("closed-"+std::to_string(cycle)+"-task-ids.txt"),closed);
        cases<<cycle<<"\t0\t3\t3\t3\t2\t3\t"<<baseline_fds<<'\t'<<fds()<<'\t'<<baseline_tasks.size()<<'\t'<<(closed==baseline_tasks)<<'\t'<<weak.expired()<<'\n';cases.flush();
        require(cases.good() && weak.expired() && fds()==baseline_fds && closed==baseline_tasks,"whole Wlr native resources survived manager");
    }
    // Preserve the original retained failed-client behavior in a separate real
    // manager. A second connect creates capture/input without reconnecting it.
    const auto missing=std::filesystem::path(argv[1]).parent_path()/"mfs-whole-wlr-missing";
    require(!std::filesystem::exists(missing),"whole Wlr missing path exists");
    {
        const auto json=configuration(missing.c_str());
        std::unique_ptr<MAA_CTRL_UNIT_NS::LinuxControlUnitAPI,decltype(&MaaLinuxControlUnitDestroy)> unit(MaaLinuxControlUnitCreate(json.c_str()),MaaLinuxControlUnitDestroy);
        require(unit && !unit->connect() && !unit->connected() && unit->get_features()==0,"whole Wlr first missing connect differs");
        const auto client=reinterpret_cast<uintptr_t>(MfsWholeLinux::client(*unit).get());require(client!=0,"whole Wlr failed client not retained");
        cv::Mat output=held.back();const auto* address=output.data;
        require(!unit->screencap(output) && output.data==address,"whole Wlr unconnected capture changed output");
        const auto before=whole_roundtrips;
        require(unit->connect() && unit->connected() && unit->get_features()==3 && whole_roundtrips==before,"whole Wlr failed client reconnected unexpectedly");
        require(reinterpret_cast<uintptr_t>(MfsWholeLinux::client(*unit).get())==client && !MfsWholeLinux::client(*unit)->connected()
            && MfsWholeLinux::wlr_type(*unit) && MfsWholeLinux::observe(*unit).input_none==1,"whole Wlr failed client owner/type differs");
        require(!unit->screencap(output) && output.data==address,"whole Wlr failed native capture changed output");pixels(output);
    }
    require(fds()==baseline_fds && tasks()==baseline_tasks,"whole Wlr missing-client resources differ");
    text(directory/"failed-client-control.txt","first_connected=0 first_features=0 retained_connected=1 retained_features=3 same_client=1 failed_output_preserved=1 resources_balanced=1\n");
    for(size_t i=0;i<held.size();++i){save(directory/("held-"+std::to_string(i)+".bgr"),held[i]);}
    std::cout<<"original whole Wlr manager objects=21 capture_objects=20 captures=60 connects=42 held=60 owner_observations=140 active_features=3 same_client_reconstruction=20 failed_client_control=1 resources_balanced=1 input_actions=0 full_P1_acceptance=0 passed\n";
    return 0;
}
}
