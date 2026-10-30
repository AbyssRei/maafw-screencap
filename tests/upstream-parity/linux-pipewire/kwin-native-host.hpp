// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Real original KWin C entry/virtual API; host authorization remains controlled.
struct MaaToolkitPortalHelper {
    int original=-1,offered=-1;
    bool transferred=false;
    ~MaaToolkitPortalHelper() {
        if(original>=0) {::close(original);}
        if(offered>=0 && !transferred) {::close(offered);}
    }
};
namespace {
struct KWinHost {
    const char* socket=nullptr;
    uint32_t node=0;
    int fd=-1;
    uint64_t before=0,after=0;
    unsigned helper_closed=0,helper_ended=0;
    std::unique_ptr<MaaToolkitPortalHelper> helper;
    std::vector<std::string> events;
} kwin_host;
void kwin_helper(MaaToolkitPortalHelper* helper) {
    require(helper && helper==kwin_host.helper.get(),"absent/replaced KWin host helper");
}
}
extern "C" MaaToolkitPortalHelper* MaaToolkitPortalHelperCreate() {
    require(!kwin_host.helper,"KWin source created overlapping helpers");
    kwin_host.events.push_back("create");
    kwin_host.helper=std::make_unique<MaaToolkitPortalHelper>();return kwin_host.helper.get();
}
extern "C" MaaBool MaaToolkitPortalHelperOpenStream(MaaToolkitPortalHelper* helper) {
    kwin_helper(helper);kwin_host.events.push_back("open");
    helper->original=connect_socket(kwin_host.socket);
    kwin_host.before=socket_cookie(helper->original);
    helper->offered=::dup(helper->original);
    require(helper->offered>=0 && socket_cookie(helper->offered)==kwin_host.before,"KWin host dup lost source identity");
    const int original=std::exchange(helper->original,-1);
    require(::close(original)==0,"cannot end original host message socket");
    errno=0;require(::fcntl(original,F_GETFD)==-1 && errno==EBADF,"original host fd survived close");
    ++kwin_host.helper_closed;return 1;
}
extern "C" int MaaToolkitPortalHelperGetPipeWireFD(MaaToolkitPortalHelper* helper) {
    kwin_helper(helper);kwin_host.events.push_back("fd");
    require(!helper->transferred && helper->offered>=0,"KWin repeated/invalid fd offer");
    helper->transferred=true;kwin_host.fd=helper->offered;return helper->offered;
}
extern "C" uint32_t MaaToolkitPortalHelperGetPipeWireNodeID(MaaToolkitPortalHelper* helper) {
    kwin_helper(helper);kwin_host.events.push_back("node");return kwin_host.node;
}
extern "C" void MaaToolkitPortalHelperDestroy(MaaToolkitPortalHelper* helper) {
    kwin_helper(helper);kwin_host.events.push_back("destroy");
    require(helper->transferred,"KWin source destroyed helper before retrieving its offer");
    kwin_host.helper.reset();++kwin_host.helper_ended;
    kwin_host.after=socket_cookie(kwin_host.fd);
}
namespace {
void kwin_save(const std::filesystem::path& path,std::string_view text) {
    require(text.size()<=static_cast<size_t>(std::numeric_limits<std::streamsize>::max()),"KWin ABI witness write span overflow");
    std::ofstream output(path,std::ios::binary);output.write(text.data(),static_cast<std::streamsize>(text.size()));
    output.close();require(output.good(),"cannot save actual KWin ABI witness");
}
void manager_kwin_native_context(const char* socket,uint32_t node,const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    require(std::string_view(MaaKWinControlUnitGetVersion())=="mfs-p1-kwin-native","wrong original KWin API version");
    MaaKWinControlUnitDestroy(nullptr);
    std::ofstream rows(directory/"source-cases.tsv"),api(directory/"source-api.tsv");
    require(rows.good() && api.good(),"cannot create KWin native records");
    cv::Mat warm(HEIGHT,WIDTH,CV_8UC3,cv::Scalar(1,2,3)),converted;cv::cvtColor(warm,converted,cv::COLOR_RGB2BGR);
    const auto base_fds=count_entries("/proc/self/fd");const auto baseline=task_ids();
    const auto save_tasks=[](const auto& path,const auto& ids) {
        std::ofstream output(path);for(const auto& id:ids) {output<<id<<'\n';}
        output.close();require(output.good(),"cannot save KWin exact task identities");
    };
    save_tasks(directory/"baseline-task-ids.txt",baseline);
    std::vector<HeldImage> held;held.reserve(60);
    for(unsigned cycle=0;cycle<20;++cycle) {
        kwin_host={};kwin_host.socket=socket;kwin_host.node=node;
        MfsOriginalPipeWire::host_input={};auto& state=MfsOriginalPipeWire::host_input;
        const unsigned outcome=cycle%3;state.enabled=true;state.outcome=outcome;
        std::vector<Task> live;int result=-1,error_code=0;uint64_t features=0;
        {
            struct Balance {~Balance() {if(MfsOriginalPipeWire::host_input.native_constructed) {pw_deinit();}}} balance;
            // Whole original compat cpp constructs the actual original manager;
            // the returned real public handle and its interface bases stay live.
            std::unique_ptr<MAA_CTRL_UNIT_NS::LinuxControlUnitAPI,decltype(&MaaKWinControlUnitDestroy)> unit(
                MaaKWinControlUnitCreate("/capture-only-host",17,19,255),MaaKWinControlUnitDestroy);
            require(unit && dynamic_cast<MAA_CTRL_UNIT_NS::ScrollableUnit*>(unit.get())
                && dynamic_cast<MAA_CTRL_UNIT_NS::RelativeMovableUnit*>(unit.get()),"KWin actual public virtual handle/bases missing");
            require(kwin_host.events==std::vector<std::string>{"create","open","fd","node","destroy"}
                && kwin_host.helper_closed==1 && kwin_host.helper_ended==1 && !kwin_host.helper
                && kwin_host.before!=0 && kwin_host.before==kwin_host.after,"original KWin host call/offer order differs");
            state.capture_fd=kwin_host.fd;
            std::string uuid;require(unit->request_uuid(uuid),"KWin virtual UUID failed");
            require(uuid==std::to_string(kwin_host.fd)+"_"+std::to_string(node)+"_/capture-only-host","KWin native UUID lost retained input/source");
            const std::string metadata=json::value(unit->get_info()).to_string();
            kwin_save(directory/("api-"+std::to_string(cycle)+".uuid"),uuid);
            kwin_save(directory/("api-"+std::to_string(cycle)+".info"),metadata);
            kwin_save(directory/("api-"+std::to_string(cycle)+".events"),"create,open,fd,node,destroy");
            cv::Mat output;require(!unit->connected() && unit->get_features()==0 && !unit->screencap(output) && output.empty(),"unconnected KWin virtual state/output differs");
            result=2;try {result=unit->connect()?1:0;}catch(const MfsOriginalPipeWire::HostInputInitFailure&) {error_code=97;}
            require(result==(outcome==0?1:outcome==1?0:2) && error_code==(outcome==2?97:0)
                && unit->connected()==(outcome==0) && unit->get_features()==3,"KWin virtual host init result/features differ");
            for(unsigned call=0;call<3;++call) {
                if(call==1) {require(unit->inactive(),"KWin virtual inactive failed");}
                require(unit->screencap(output),"KWin virtual capture forwarding lost retained native owner");
                save(output,directory/("host-"+std::to_string(cycle)+"-call-"+std::to_string(call)+".bgr"));
                const auto serial=known_pixels(output);held.push_back({std::move(output),serial});
                for(const auto& image:held) {require(known_pixels(image.image)==image.marker,"KWin held owner changed during later captures");}
            }
            require(json::value(unit->get_info()).to_string()==metadata,"KWin connect/inactive changed caller metadata");
            features=unit->get_features();live=live_tasks();
        }
        errno=0;const bool socket_closed=::fcntl(kwin_host.fd,F_GETFD)==-1 && errno==EBADF;
        const auto fds=count_entries("/proc/self/fd");const auto immediate=task_ids();bool settled=immediate==baseline;
        if(fds==base_fds && !settled && only_retiring_tasks(immediate,baseline,live)) {
            const auto start=std::chrono::steady_clock::now();
            for(;;) {
                const auto current=task_ids();if(current==baseline) {settled=true;break;}
                if(!only_retiring_tasks(current,baseline,live) || std::chrono::steady_clock::now()-start>=std::chrono::milliseconds(100)) {break;}
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
        }
        rows<<cycle<<'\t'<<outcome<<'\t'<<result<<'\t'<<error_code<<'\t'<<state.created<<'\t'<<state.initialized<<'\t'<<state.inactive<<'\t'<<state.destroyed
            <<'\t'<<state.native_closed_before_input<<'\t'<<state.host_fds_closed<<'\t'<<state.errors<<'\t'<<socket_closed<<'\t'<<base_fds<<'\t'<<fds
            <<'\t'<<baseline.size()<<'\t'<<settled<<'\t'<<state.capture_observation_error<<'\t'<<state.close_errors[0]<<'\t'<<state.close_errors[1]<<'\t'<<features
            <<'\t'<<kwin_host.helper_ended<<'\t'<<(kwin_host.before==kwin_host.after)<<'\n';rows.flush();
        api<<cycle<<'\t'<<kwin_host.fd<<'\t'<<node<<"\t1\t1\t"<<kwin_host.helper_closed<<'\t'<<kwin_host.before<<'\t'<<kwin_host.after<<"\t5\n";api.flush();
        const auto closed=task_ids();save_tasks(directory/("closed-"+std::to_string(cycle)+"-task-ids.txt"),closed);
        require(rows.good() && api.good() && socket_closed && state.created==1 && state.initialized==1 && state.inactive==1 && state.destroyed==1
            && state.native_closed_before_input==1 && state.host_fds_closed==2 && state.errors==0 && fds==base_fds && settled && closed==baseline,"KWin virtual destruction/resources differ");
        state.enabled=false;
    }
    for(size_t index=0;index<held.size();++index) {
        require(known_pixels(held[index].image)==held[index].marker,"KWin held pixels changed after virtual destruction");
        save(held[index].image,directory/("held-"+std::to_string(index)+".source.bgr"));
    }
    std::cout<<"original KWin ABI objects=20 host_true_false_error=7,7,6 captures=60 retained=60 exact_compat_create_destroy=20 real_virtual_handles=20 helper_call_order=20 duplicated_socket_identity=20 input_actions=0 passed\n";
}
}
