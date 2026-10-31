// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Prefix observation of unchanged WaylandClient::close. The fixture delays
// only its final disconnect when old capture snapshots are still alive.
// This does not execute invalid proxy destructors after a real disconnect or
// claim that the delayed native lifetime is the original close behavior.
struct CloseObservation {
    MfsOriginalWlr::WaylandClient* client=nullptr;
    std::array<std::weak_ptr<void>,3> snapshots;
    std::array<long,3> references{};
    wl_display* pending=nullptr;
    unsigned calls=0,globals=0,connected=0,live_fd=0,deferred=0,forwarded=0;
    int fd=-1;
    bool error=false;
    ~CloseObservation() {
        // Declared before client/captures in the fixture: their normal or
        // exceptional destruction ends all proxy owners before this fallback.
        if(pending) {__real_wl_display_disconnect(pending);}
    }
    void observe(wl_display* display) noexcept {
        ++calls;
        if(calls!=1 || pending || !client) {error=true;return;}
        connected=client->connected();
        globals=(client->get_screencopy_manager()?1U:0U)
            |(client->get_shm()?2U:0U)|(client->get_output()?4U:0U);
        for(size_t i=0;i<snapshots.size();++i) {references[i]=snapshots[i].use_count();}
        fd=wl_display_get_fd(display);
        live_fd=fd>=0 && fcntl(fd,F_GETFD)>=0;
        if(std::any_of(references.begin(),references.end(),[](long n){return n!=0;})) {
            pending=display;deferred=1;
        } else {__real_wl_display_disconnect(display);++forwarded;}
    }
    bool closed() const noexcept {errno=0;return fcntl(fd,F_GETFD)<0 && errno==EBADF;}
    void finish() {
        require(!error && calls==1,"early-close disconnect observation failed");
        for(const auto& snapshot:snapshots) {require(snapshot.expired(),"deferred disconnect still has native snapshot owners");}
        if(pending) {
            require(fcntl(fd,F_GETFD)>=0,"fixture lost deferred native connection");
            auto* display=std::exchange(pending,nullptr);
            __real_wl_display_disconnect(display);++forwarded;
        }
        require(forwarded==1 && closed(),"fixture did not finish exactly one native disconnect");
    }
};
CloseObservation* close_observer=nullptr;
struct ObserveClose {
    explicit ObserveClose(CloseObservation& state) {
        require(close_observer==nullptr,"nested disconnect observer");close_observer=&state;
    }
    ~ObserveClose() {close_observer=nullptr;}
};

void early_close_context(const std::filesystem::path& socket,const std::filesystem::path& directory) {
    struct Alias {
        std::filesystem::path path;
        ~Alias() {std::error_code error;std::filesystem::remove(path,error);}
    } alias{directory/"early-close-alias"};
    require(!std::filesystem::exists(alias.path),"early-close alias already exists");
    std::filesystem::create_symlink(socket,alias.path);
    const auto baseline_tasks=tasks();
    std::ofstream baseline(directory/"early-close-tasks.txt");
    for(const auto& id:baseline_tasks) {baseline<<id<<'\n';}
    baseline.close();require(baseline.good(),"cannot save early-close task identities");
    std::ofstream rows(directory/"early-close.tsv");
    rows<<"cycle\tmode\tcalls\tdeferred\tmanager_refs\tshm_refs\toutput_refs\tglobals\tconnected\tfd_live\tclosed_capture_preserved\trebound\tsnapshots_expired\tforwarded\tfd_closed\tfds_before\tfds_after\ttasks_equal\theld\n";
    const auto baseline_fds=count("/proc/self/fd");
    std::vector<std::pair<std::string,cv::Mat>> held;held.reserve(60);
    const std::array<const char*,4> modes={"control","cold","buffered","reconnect"};
    for(int cycle=0;cycle<20;++cycle) {for(size_t mode=0;mode<modes.size();++mode) {
        CloseObservation state;
        auto client=std::make_shared<MfsOriginalWlr::WaylandClient>();
        require(client->connect(socket),"early-close original connect failed");
        state.client=client.get();
        state.snapshots={client->get_screencopy_manager(),client->get_shm(),client->get_output()};
        std::unique_ptr<MfsOriginalWlr::WlrScreencap> source;
        const auto name=std::to_string(cycle)+"-"+modes[mode];
        const auto before_held=held.size();
        if(mode!=0) {source=std::make_unique<MfsOriginalWlr::WlrScreencap>(client);}
        if(mode>=2) {
            cv::Mat image;require(source->screencap(image),"early-close original capture failed");
            save(image,directory/(name+"-before.bgr"));pixels(image);held.emplace_back(name+"-before",std::move(image));
        }
        unsigned preserved=0,rebound=0;
        {
            ObserveClose observe(state);
            if(mode==3) {require(client->connect(alias.path),"live-snapshot original reconnect failed");rebound=1;}
            else {client->close();}
        }
        require(!state.error && state.calls==1 && state.globals==0 && state.connected==0 && state.live_fd==1,
            "original close prefix did not clear its globals before disconnect");
        for(long refs:state.references) {require(refs==(mode==0?0:1),"old capture snapshot references differ");}
        require(state.deferred==(mode==0?0U:1U),"fixture disconnect classification differs");
        if(mode==1 || mode==2) {
            require(!client->connected(),"original close remained connected");
            cv::Mat output(48,64,CV_8UC3,cv::Scalar(11,29,47));
            auto* data=output.data;auto* owner=output.u;const auto before=output.clone();
            require(!source->screencap(output) && output.data==data && output.u==owner
                && cv::norm(output,before,cv::NORM_INF)==0,"closed original capture changed caller output");
            preserved=1;
        } else if(mode==3) {
            require(client->connected(),"original reconnect did not restore client state");
            const std::array<std::weak_ptr<void>,3> fresh={client->get_screencopy_manager(),client->get_shm(),client->get_output()};
            for(size_t i=0;i<fresh.size();++i) {
                require(!fresh[i].expired() && (fresh[i].owner_before(state.snapshots[i]) || state.snapshots[i].owner_before(fresh[i])),
                    "reconnect reused the old native snapshot owner");
            }
            // The old source still points at the old proxies while its shared
            // client now dispatches the new display. Do not call that invalid
            // combination. Its behavior remains outside this prefix probe.
        }
        source.reset();
        for(const auto& snapshot:state.snapshots) {require(snapshot.expired(),"original source destructor retained old snapshot");}
        state.finish();
        if(mode==3) {
            MfsOriginalWlr::WlrScreencap fresh(client);
            cv::Mat image;require(fresh.screencap(image),"fresh source after reconnect failed");
            save(image,directory/(name+"-fresh.bgr"));pixels(image);held.emplace_back(name+"-fresh",std::move(image));
        }
        client->close();client.reset();
        const auto final_fds=count("/proc/self/fd");const auto final_tasks=tasks();
        std::ofstream task_file(directory/(name+".tasks"));
        for(const auto& id:final_tasks) {task_file<<id<<'\n';}
        task_file.close();require(task_file.good(),"cannot save final early-close tasks");
        for(const auto& entry:held) {pixels(entry.second);}
        rows<<cycle<<'\t'<<modes[mode]<<'\t'<<state.calls<<'\t'<<state.deferred;
        for(long refs:state.references) {rows<<'\t'<<refs;}
        rows<<'\t'<<state.globals<<'\t'<<state.connected<<'\t'<<state.live_fd<<'\t'<<preserved<<'\t'<<rebound
            <<"\t1\t"<<state.forwarded<<"\t1\t"<<baseline_fds<<'\t'<<final_fds<<'\t'<<(final_tasks==baseline_tasks)<<'\t'<<(held.size()-before_held)<<'\n';
        rows.flush();require(rows.good(),"cannot save early-close native observations");
        require(final_fds==baseline_fds && final_tasks==baseline_tasks,"early-close fixture native resources differ");
    }}
    for(const auto& [name,image]:held) {pixels(image);save(image,directory/(name+".held"));}
    rows.close();require(rows.good() && held.size()==60,"early-close evidence incomplete");
    std::cout<<"original Wlr early_close cycles=20 cases=80 disconnect_prefixes=80 deferred_by_fixture=60 closed_capture_preserved=40 rebound=20 captures=60 retained=60 resources_balanced=1 passed\n";
}
