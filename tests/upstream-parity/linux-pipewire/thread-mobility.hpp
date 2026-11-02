// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included after save/source/task helpers. Calls the unchanged complete F class.
struct MobilityGate {
    std::mutex mutex;
    std::condition_variable changed;
    bool opened=false;
    void open() {std::lock_guard lock(mutex);opened=true;changed.notify_all();}
    void wait() {std::unique_lock lock(mutex);changed.wait(lock,[&]{return opened;});}
};
struct MobilityRelease {
    MobilityGate& gate;
    ~MobilityRelease() {gate.open();}
};
void mobility_ids(const std::filesystem::path& path,const std::vector<std::string>& ids) {
    std::ofstream out(path);out<<"id\n";for(const auto& id:ids) {out<<id<<'\n';}
    out.close();require(out.good(),"cannot save mobility task IDs");
}
void mobility_task_record(const std::filesystem::path& path,const std::vector<Task>& live,
    const std::vector<std::string>& baseline) {
    std::ofstream out(path);out<<"id\tname\tstarted\tflags\taddress_space\n";
    size_t count=0;
    for(const auto& task:live) {
        if(std::find(baseline.begin(),baseline.end(),task.id)!=baseline.end()) {continue;}
        out<<task.id<<'\t'<<task.name<<'\t'<<task.started<<'\t'<<task.flags<<'\t'<<task.address_space<<'\n';++count;
    }
    out.close();require(out.good() && count==4,"cannot save four owned live task generations");
}
void thread_mobility_context(const char* socket,uint32_t node,bool supplied,const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    const auto baseline=task_ids();
    std::vector<HeldImage> held;held.reserve(60);
    std::ofstream rows(directory/"rows.tsv");
    rows<<"cycle\tcreator_tid\tcapture_tid\tcloser_tid\tloop_tid\tloop_started\tfd_before\tfd_after\ttasks_before\ttasks_after\toffered_fd\toffered_fd_closed\tcaptures\tretained_frames\n";
    require(rows.good(),"cannot create original mobility rows");
    const size_t base_fds=count_entries("/proc/self/fd"); // includes this persistent record owner
    mobility_ids(directory/"baseline.tsv",baseline);
    for(size_t cycle=0;cycle<20;++cycle) {
        struct Context {
            std::unique_ptr<Source,SourceOwner> source;
            std::array<std::atomic<uint64_t>,3> ids{};
            std::array<std::atomic<int>,3> names{};
            std::array<std::exception_ptr,3> errors{};
            std::array<cv::Mat,3> images;
            std::array<uint32_t,3> markers{};
            std::vector<Task> live;
            uint64_t loop=0,started=0;
            int offered=-1;
            bool offered_closed=false;
        } context;
        MobilityGate start,finished;
        std::latch ready(3);
        std::binary_semaphore to_capture(0),to_close(0);
        {
            // On a thread-construction exception the gates open BEFORE worker
            // join/destruction. No worker waits on a missing ready participant.
            std::vector<std::jthread> workers;workers.reserve(3);
            MobilityRelease start_on_exit{start},finish_on_exit{finished};
            auto identify=[&](size_t slot) {
                context.ids[slot].store(static_cast<uint64_t>(gettid()));
                // The existing strict post-join observer recognizes these four
                // explicitly owned/joined generations. Roles come from actual
                // caller TIDs and handoffs, never from inferring role by name.
                context.names[slot].store(pthread_setname_np(pthread_self(),"MaaScreencap"));
                ready.count_down();start.wait();
            };
            auto live_loop=[&] {
                const auto value=task(std::to_string(context.loop));
                require(value && value->started==context.started && known_live_source_task(*value),
                    "native loop generation changed across caller handoff");
            };
            workers.emplace_back([&] {
                identify(0);
                try {
                    for(size_t i=0;i<3;++i) {
                        require(context.ids[i].load()>0 && context.names[i].load()==0,"caller identity unavailable");
                        for(size_t j=0;j<i;++j) {require(context.ids[i].load()!=context.ids[j].load(),"callers are not distinct live threads");}
                    }
                    context.offered=supplied ? connect_socket(socket) : -1;
                    Source* created=nullptr;
                    try {created=new Source(context.offered,node);}
                    catch(...) {if(context.offered>=0) {close(context.offered);}throw;}
                    context.source.reset(created);
                    require(context.source->init() && context.source->connected(),"creator-thread original init failed");
                    context.live=live_tasks();
                    require(context.live.size()==baseline.size()+4,"unexpected caller/native task population");
                    for(const auto& task:context.live) {
                        if(std::find(baseline.begin(),baseline.end(),task.id)!=baseline.end()) {continue;}
                        const bool caller=std::any_of(context.ids.begin(),context.ids.end(),[&](const auto& id){return task.id==std::to_string(id.load());});
                        require(known_live_source_task(task),"owned generation is not a live named task");
                        if(!caller) {require(context.loop==0,"multiple native loop tasks");context.loop=std::stoull(task.id);context.started=task.started;}
                    }
                    require(context.loop>0 && context.started>0,"native loop identity absent");
                }catch(...) {context.errors[0]=std::current_exception();}
                to_capture.release();finished.wait();
            });
            workers.emplace_back([&] {
                identify(1);to_capture.acquire();
                try {
                    if(!context.errors[0]) {
                        live_loop();
                        for(size_t call=0;call<2;++call) {
                            if(call==1) {context.source->inactive();}
                            require(context.source->screencap(context.images[call]),"capture-thread original capture failed");
                            save(context.images[call],directory/("cycle-"+std::to_string(cycle)+"-call-"+std::to_string(call)+".bgr"));
                            context.markers[call]=known_pixels(context.images[call]);
                        }
                    }
                }catch(...) {context.errors[1]=std::current_exception();}
                to_close.release();finished.wait();
            });
            workers.emplace_back([&] {
                identify(2);MobilityRelease release{finished};to_close.acquire();
                try {
                    if(!context.errors[0] && !context.errors[1]) {
                        live_loop();
                        require(context.source->screencap(context.images[2]),"closer-thread original capture failed");
                        save(context.images[2],directory/("cycle-"+std::to_string(cycle)+"-call-2.bgr"));
                        context.markers[2]=known_pixels(context.images[2]);
                        context.source->close();
                        require(!context.source->connected(),"closed source remained connected");
                    }
                }catch(...) {context.errors[2]=std::current_exception();}
                context.source.reset(); // actual original destruction on caller3
                if(supplied && context.offered>=0) {
                    errno=0;context.offered_closed=fcntl(context.offered,F_GETFD)==-1 && errno==EBADF;
                }
            });
            ready.wait();start.open();
            for(auto& worker:workers) {worker.join();}
        }
        for(const auto& error:context.errors) {if(error) {std::rethrow_exception(error);}}
        require(!supplied || context.offered_closed,"offered source socket was not closed before artifact I/O");
        const size_t fds=count_entries("/proc/self/fd");
        const auto immediate=task_ids();bool settled=immediate==baseline;
        if(fds==base_fds && !settled && only_retiring_tasks(immediate,baseline,context.live)) {
            task_snapshot(directory,"mobility-"+std::to_string(cycle)+"-retiring",immediate);
            const auto start=std::chrono::steady_clock::now();
            for(;;) {
                const auto current=task_ids();
                if(current==baseline) {settled=true;break;}
                if(!only_retiring_tasks(current,baseline,context.live) ||
                    std::chrono::steady_clock::now()-start>=std::chrono::milliseconds(100)) {break;}
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
        }
        if(fds!=base_fds || !settled) {task_snapshot(directory,"mobility-"+std::to_string(cycle)+"-mismatch",immediate);}
        require(fds==base_fds && settled,"thread-mobility original resource mismatch");
        mobility_task_record(directory/("cycle-"+std::to_string(cycle)+"-live.tsv"),context.live,baseline);
        mobility_ids(directory/("cycle-"+std::to_string(cycle)+"-final.tsv"),task_ids());
        for(size_t call=0;call<3;++call) {
            require(known_pixels(context.images[call])==context.markers[call],"handoff/close changed original CPU output");
            save(context.images[call],directory/("cycle-"+std::to_string(cycle)+"-held-"+std::to_string(call)+".bgr"));
            held.push_back({std::move(context.images[call]),context.markers[call]});
        }
        rows<<cycle<<'\t'<<context.ids[0].load()<<'\t'<<context.ids[1].load()<<'\t'<<context.ids[2].load()<<'\t'
            <<context.loop<<'\t'<<context.started<<'\t'<<base_fds<<'\t'<<fds<<'\t'<<baseline.size()<<'\t'<<task_ids().size()
            <<'\t'<<context.offered<<'\t'<<context.offered_closed<<"\t3\t3\n";
        require(rows.good(),"cannot save original mobility row");
    }
    rows.close();require(rows.good(),"cannot close original mobility rows");
    for(const auto& image:held) {require(known_pixels(image.image)==image.marker,"later lifecycles changed held original pixels");}
    std::cout<<"original PipeWire thread mobility mode="<<supplied<<" objects=20 callers=3 captures=60 retained=60 fd_tasks_balanced=20 passed\n";
}
