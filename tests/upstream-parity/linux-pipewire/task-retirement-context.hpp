// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included inside the source caller's private namespace. These are causal
// kernel/procfs controls, without a capture backend or changed capture wait.
void task_retirement_read_fixture(const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    const auto baseline_fds=count_entries("/proc/self/fd");
    const auto baseline_tasks=task_ids();
    int synthetic_negatives=0;
    TaskAbsence retired_probe,live_probe,limited_probe;
    {
        struct Worker {
            std::latch ready{1},release{1};
            pid_t tid=-1;
            int name_error=0;
            bool released=false,joined=false;
            std::thread thread;
            Worker():thread([this]{
                tid=gettid();name_error=pthread_setname_np(pthread_self(),"MaaScreencap");
                ready.count_down();release.wait();
            }) {}
            void finish() {
                if(!released) {released=true;release.count_down();}
                if(thread.joinable()) {thread.join();joined=true;}
            }
            ~Worker() {finish();}
        } retired,live;
        retired.ready.wait();live.ready.wait();
        require(retired.name_error==0 && live.name_error==0,"cannot name controlled source-task fixtures");
        const auto id=std::to_string(retired.tid);
        const auto before=task(id),live_before=task(std::to_string(live.tid));
        require(before && live_before && known_live_source_task(*before) && known_live_source_task(*live_before),
            "missing known-live fixture identities");
        const auto path=std::filesystem::path("/proc/self/task")/id/"stat";
        std::ifstream input(path);
        require(input.good(),"cannot open held live stat stream for retirement fixture");
        retired.finish();require(retired.joined,"source-task fixture was not joined before read");
        // Select a known retired task for this causal control only. The actual
        // source resource gate retains its existing 100ms bound and no new wait.
        const auto deadline=std::chrono::steady_clock::now()+std::chrono::seconds(2);
        while(std::filesystem::exists(path) && std::chrono::steady_clock::now()<deadline) {std::this_thread::yield();}
        require(!std::filesystem::exists(path),"controlled joined task did not retire");
        std::optional<TaskStatReadFailure> failure;
        try {static_cast<void>(task_text(input,id));}
        catch(const TaskStatReadFailure& error) {
            failure.emplace(error.id,error.native_error,error.bad,error.failed,error.eof,error.bytes);
        }
        require(failure && exact_retired_read_failure(*before,*failure),"held retired task did not produce exact native ESRCH evidence");
        require(observe_post_join_failed_task(*before,*failure,retired_probe) && proved_task_absent(retired_probe),
            "known joined task lacked fresh stat/directory absence proof");

        // A real live path must refuse the same read-error signature. The live
        // failure metadata is deliberately synthetic; the fresh native open is
        // real, and its successful descriptor is closed before refusal.
        const TaskStatReadFailure synthetic_live(live_before->id,ESRCH,true,true,false,0);
        require(!observe_post_join_failed_task(*live_before,synthetic_live,live_probe) && live_probe.stat_opened && live_probe.close_error==0,
            "retirement classifier accepted a real live task path");

        struct LimitedDescriptors {
            rlimit previous{};
            bool changed=false;
            int close_errors=0,restore_error=0;
            std::vector<int> fds;
            void restore() noexcept {
                for(int fd:fds) {if(close(fd)!=0) {++close_errors;}}
                fds.clear();
                if(changed && setrlimit(RLIMIT_NOFILE,&previous)!=0) {restore_error=errno;}
                changed=false;
            }
            ~LimitedDescriptors() {restore();}
        } limit;
        limit.fds.reserve(128);
        require(getrlimit(RLIMIT_NOFILE,&limit.previous)==0,"cannot read fixture fd limit");
        auto selected=limit.previous;
        selected.rlim_cur=std::min(selected.rlim_cur,static_cast<rlim_t>(64));
        require(selected.rlim_cur>=32 && baseline_fds<selected.rlim_cur-8,"fd limit too small for the isolated causal fixture");
        require(setrlimit(RLIMIT_NOFILE,&selected)==0,"cannot select isolated fixture fd limit");limit.changed=true;
        for(;;) {
            const int fd=open("/dev/null",O_RDONLY|O_CLOEXEC);
            if(fd<0) {require(errno==EMFILE,"descriptor exhaustion did not report real EMFILE");break;}
            limit.fds.push_back(fd);
            require(limit.fds.size()<128,"controlled fd exhaustion exceeded its explicit bound");
        }
        require(!observe_post_join_failed_task(*before,*failure,limited_probe) && !limited_probe.stat_opened && limited_probe.stat_error==EMFILE,
            "retirement classifier accepted an unproved path after real fresh-open EMFILE");
        limit.restore();require(limit.close_errors==0 && limit.restore_error==0,"cannot restore fixture descriptors/limit");
        rlimit restored{};require(getrlimit(RLIMIT_NOFILE,&restored)==0 && restored.rlim_cur==limit.previous.rlim_cur &&
            restored.rlim_max==limit.previous.rlim_max,"fixture fd limit changed after restoration");

        // Labelled synthetic metadata controls do not stand in for native
        // teardown. Positive native ESRCH/live-path/EMFILE controls are above.
        auto reject_before=[&](Task altered) {
            require(!exact_retired_read_failure(altered,*failure),"unsafe synthetic before identity accepted");++synthetic_negatives;
        };
        {auto altered=*before;altered.name="unknown";reject_before(std::move(altered));}
        {auto altered=*before;altered.started=0;reject_before(std::move(altered));}
        {auto altered=*before;altered.flags|=0x4;reject_before(std::move(altered));}
        {auto altered=*before;altered.address_space=0;reject_before(std::move(altered));}
        {auto altered=*before;altered.id="";reject_before(std::move(altered));}
        {auto altered=*before;altered.id="../invalid";reject_before(std::move(altered));}
        auto reject_read=[&](const TaskStatReadFailure& altered) {
            require(!exact_retired_read_failure(*before,altered),"unsafe synthetic read evidence accepted");++synthetic_negatives;
        };
        reject_read(TaskStatReadFailure("different",ESRCH,true,true,false,0));
        reject_read(TaskStatReadFailure(id,EIO,true,true,false,0));
        reject_read(TaskStatReadFailure(id,ENOENT,true,true,false,0));
        reject_read(TaskStatReadFailure(id,ESRCH,false,true,false,0));
        reject_read(TaskStatReadFailure(id,ESRCH,true,false,false,0));
        reject_read(TaskStatReadFailure(id,ESRCH,true,true,true,0));
        reject_read(TaskStatReadFailure(id,ESRCH,true,true,false,1));
        auto reject_probe=[&](TaskAbsence altered) {
            require(!proved_task_absent(altered),"unsafe synthetic fresh-probe evidence accepted");++synthetic_negatives;
        };
        {auto altered=retired_probe;altered.stat_opened=true;reject_probe(altered);}
        {auto altered=retired_probe;altered.stat_error=EACCES;reject_probe(altered);}
        {auto altered=retired_probe;altered.close_error=EIO;reject_probe(altered);}
        {auto altered=retired_probe;altered.directory_exists=true;reject_probe(altered);}
        {auto altered=retired_probe;altered.directory_error=EACCES;reject_probe(altered);}
        Task exiting=*before;exiting.flags|=0x4;exiting.address_space=0;exiting.resident_pages=0;
        require(same_generation_kernel_retirement(*before,exiting),"same-generation synthetic exiting premise failed");
        auto reject_after=[&](Task altered) {
            require(!same_generation_kernel_retirement(*before,altered),"live/replaced synthetic task accepted");++synthetic_negatives;
        };
        {auto altered=exiting;altered.started^=1;reject_after(std::move(altered));}
        {auto altered=exiting;altered.name="other";reject_after(std::move(altered));}
        {auto altered=exiting;altered.id="other";reject_after(std::move(altered));}
        {auto altered=exiting;altered.flags&=~uint64_t{0x4};reject_after(std::move(altered));}
        {auto altered=exiting;altered.address_space=1;reject_after(std::move(altered));}
        {auto altered=exiting;altered.resident_pages=1;reject_after(std::move(altered));}
        require(synthetic_negatives==24,"unexpected synthetic refusal-control count");
        require(!only_retiring_tasks({id},{},{}) && !only_retiring_tasks({},baseline_tasks,{*before}),
            "unknown task or missing baseline identity accepted");
        input.clear();input.close();require(!input.fail(),"cannot close held retired-task stream");
        live.finish();require(live.joined,"live negative-control task did not join");
    }
    require(count_entries("/proc/self/fd")==baseline_fds && task_ids()==baseline_tasks,
        "retirement read fixture did not restore exact fd/task identities");
    std::ofstream saved(directory/"observations.txt");
    saved<<"native_held_stat_ESRCH=1\njoined_before_read=1\nknown_source_identity=1\n"
        <<"fresh_stat_errno="<<retired_probe.stat_error<<"\nfresh_directory_errno="<<retired_probe.directory_error
        <<"\nreal_live_path_refused=1\nreal_EMFILE="<<limited_probe.stat_error
        <<"\nsynthetic_metadata_negatives="<<synthetic_negatives<<"\nfd_limit_restored=1\nresources_balanced=1\n";
    saved.close();require(saved.good(),"cannot save retirement read fixture observations");
    std::cout<<"native task retirement-read exact_ESRCH=1 joined_before_read=1 fresh_stat_and_directory_absent=1"
        <<" real_live_path_refused=1 real_EMFILE_refused=1 synthetic_metadata_negatives="<<synthetic_negatives
        <<" fd_limit_restored=1 resources_balanced=1 passed\n";
}
