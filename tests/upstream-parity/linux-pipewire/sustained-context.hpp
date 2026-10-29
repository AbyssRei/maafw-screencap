// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Test caller only; complete fixed manager/capture bodies remain unchanged.
uint64_t process_rss_kib() {
    std::ifstream input("/proc/self/statm");
    uint64_t size=0,resident=0;
    require(static_cast<bool>(input>>size>>resident),"cannot inspect process RSS");
    const long page_size=sysconf(_SC_PAGESIZE);
    require(page_size>0 && page_size%1024==0,"invalid native page size");
    const auto page_kib=static_cast<uint64_t>(page_size/1024);
    require(resident<=std::numeric_limits<uint64_t>::max()/page_kib,"RSS overflow");
    return resident*page_kib;
}
void sustained_manager_context(uint32_t node,const std::filesystem::path& artifacts) {
    require(node!=0,"sustained source requires an explicit owned node");
    const char* remote=std::getenv("PIPEWIRE_REMOTE");
    const char* runtime=std::getenv("PIPEWIRE_RUNTIME_DIR");
    const char* socket=std::getenv("MFS_P0_PIPEWIRE_SOCKET");
    require(remote && runtime && socket && std::filesystem::path(runtime)/remote==std::filesystem::path(socket),
        "sustained source requires the explicitly owned private daemon");
    require(!std::filesystem::exists(artifacts),"refusing to replace a sustained result");
    std::filesystem::create_directories(artifacts/"frames");
    std::ofstream captures(artifacts/"captures.tsv"),resources(artifacts/"resources.tsv");
    require(captures.good() && resources.good(),"cannot open sustained evidence");
    captures<<"index\telapsed_ns\tcapture_ns\tmarker\tmarker_changed\treplayed\n";
    resources<<"phase\telapsed_ns\tcalls\tfds\ttasks\trss_kib\n";
    cv::Mat warm(HEIGHT,WIDTH,CV_8UC3,cv::Scalar(1,2,3)),converted;
    cv::cvtColor(warm,converted,cv::COLOR_RGB2BGR);
    const size_t base_fds=count_entries("/proc/self/fd");
    const auto baseline=task_ids();
    {
        std::ofstream identities(artifacts/"baseline-task-ids.txt");
        for(const auto& id:baseline) {identities<<id<<'\n';}
        identities.close();require(identities.good(),"cannot archive baseline task identities");
    }
    std::vector<HeldImage> held;held.reserve(3);
    std::optional<HeldImage> last;
    uint64_t calls=0,steady_ns=0;
    std::chrono::steady_clock::time_point start;
    auto record=[&](const char* phase,uint64_t elapsed) {
        resources<<phase<<'\t'<<elapsed<<'\t'<<calls<<'\t'<<count_entries("/proc/self/fd")
            <<'\t'<<task_ids().size()<<'\t'<<process_rss_kib()<<'\n';
        resources.flush();captures.flush();
        require(resources.good() && captures.good(),"cannot archive sustained resources");
    };
    std::vector<Task> live;
    {
        struct Balance {bool initialized=false;~Balance(){if(initialized) {pw_deinit();}}} balance;
        MfsOriginalPipeWire::LinuxControlUnitConfig config;
        config.screencap_method=MaaLinuxScreencapMethod_PipeWire;config.pw_node_id=node;
        MfsOriginalPipeWire::LinuxControlUnitMgr manager(config);
        const bool connected=manager.connect();balance.initialized=true;
        require(connected && manager.connected(),"sustained original manager connect failed");
        for(int i=0;i<3;++i) {
            cv::Mat image;require(manager.screencap(image),"sustained original warmup failed");known_pixels(image);
        }
        start=std::chrono::steady_clock::now();
        auto next_resource=std::chrono::nanoseconds::zero();
        std::optional<uint32_t> previous;
        while(std::chrono::steady_clock::now()-start<std::chrono::seconds(120)) {
            const auto capture_start=std::chrono::steady_clock::now();
            cv::Mat image;require(manager.screencap(image),"sustained original capture failed");
            const auto capture_ns=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-capture_start).count();
            std::ostringstream stem;stem<<"frame-"<<std::setfill('0')<<std::setw(6)<<calls;
            save(image,artifacts/"frames"/(stem.str()+".bgr"));
            const auto marker=known_pixels(image);
            require(image.u && image.u->refcount==1,"sustained source output is not independently owned");
            const auto elapsed=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-start);
            const int changed=previous?static_cast<int>(*previous!=marker):-1;
            captures<<calls<<'\t'<<elapsed.count()<<'\t'<<capture_ns<<'\t'<<marker<<'\t'<<changed<<"\t-1\n";
            previous=marker;
            require(calls<std::numeric_limits<uint64_t>::max(),"sustained call count overflow");++calls;
            if(held.size()<3) {held.push_back({std::move(image),marker});}
            else {last=HeldImage{std::move(image),marker};}
            if(elapsed>=next_resource) {
                for(const auto& item:held) {require(known_pixels(item.image)==item.marker,"sustained held source pixels changed");}
                if(last) {require(known_pixels(last->image)==last->marker,"sustained last source pixels changed");}
                record("stream",static_cast<uint64_t>(elapsed.count()));
                next_resource=elapsed+std::chrono::seconds(1);
            }
        }
        steady_ns=static_cast<uint64_t>(std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-start).count());
        require(steady_ns>=120000000000ULL && calls>=4,"sustained duration/capture requirement not met");
        record("stream",steady_ns);
        live=live_tasks();
    }
    const auto immediate=task_ids();
    bool settled=immediate==baseline;
    if(count_entries("/proc/self/fd")==base_fds && !settled && only_retiring_tasks(immediate,baseline,live)) {
        task_snapshot(artifacts,"sustained-kernel-retirement",immediate);
        const auto start=std::chrono::steady_clock::now();
        for(;;) {
            const auto current=task_ids();
            if(current==baseline) {settled=true;break;}
            if(!only_retiring_tasks(current,baseline,live) || std::chrono::steady_clock::now()-start>=std::chrono::milliseconds(100)) {break;}
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
    }
    require(count_entries("/proc/self/fd")==base_fds && settled,"sustained source resource mismatch or unretired/unknown/live task");
    {
        const auto closed_tasks=task_ids();require(closed_tasks==baseline,"sustained closed task identities changed");
        std::ofstream identities(artifacts/"closed-task-ids.txt");
        for(const auto& id:closed_tasks) {identities<<id<<'\n';}
        identities.close();require(identities.good(),"cannot archive closed task identities");
    }
    record("closed",static_cast<uint64_t>(std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-start).count()));
    for(const auto& item:held) {require(known_pixels(item.image)==item.marker,"closed sustained source changed held pixels");}
    require(last && known_pixels(last->image)==last->marker,"closed sustained source changed last pixels");
    captures.close();resources.close();
    require(captures.good() && resources.good(),"cannot close sustained evidence");
    std::ofstream summary(artifacts/"summary.txt");
    summary<<"trace_version=1\nwarmup=3\nsteady_calls="<<calls<<"\nsteady_elapsed_ns="<<steady_ns
        <<"\nretained_at_shutdown=4\nbaseline_fds="<<base_fds<<"\nerrors=0\nmarker_changes_are_not_native_freshness=1\n"
        <<"native_buffer_counters=not_exposed_by_original_API\nreplayed=-1_unknown_not_exposed_by_original_API\ngraphics=owned_CPU_RGB_fixture_no_GPU_claim\nresources_balanced=1\n";
    summary.close();require(summary.good(),"cannot archive sustained summary");
    std::cout<<"original PipeWire sustained warmup=3 steady_ns="<<steady_ns<<" calls="<<calls<<" retained=4 errors=0 resources_balanced=1 passed\n";
}
