// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Exact original KWin creation fails before manager/native capture construction.
// Controlled host helpers own real private-daemon sockets, not DBus grants.
namespace {
void kwin_portal_failure(const char* socket,const std::filesystem::path& directory) {
    require(std::filesystem::create_directory(directory),"portal failure output must be new");
    std::ofstream rows(directory/"cases.tsv");
    rows.exceptions(std::ios::badbit|std::ios::failbit);
    rows<<"index\tstage\terror\thelper_live\toriginal_closed\thelper_ended\tfd\tcookie_before\tcookie_after\tfd_calls\tnode_calls\tdestroy_calls\tcreated\tfds_before\tfds_residual\tfds_final\ttasks\tcaller\tfixture_closed\tfd_closed\thelper_final\tnative_constructed\tevents\n";
    const auto baseline=task_ids();
    const auto save_tasks=[](const auto& path,const auto& ids) {
        std::ofstream output(path);output.exceptions(std::ios::badbit|std::ios::failbit);
        for(const auto& id:ids) {output<<id<<'\n';}output.close();
    };
    save_tasks(directory/"baseline-task-ids.txt",baseline);
    const auto baseline_fds=count_entries("/proc/self/fd");
    for(unsigned index=0;index<100;++index) {
        const unsigned stage=index%5;
        kwin_host={};kwin_host.socket=socket;kwin_host.failure=stage+1;
        MfsOriginalPipeWire::host_input={};
        auto& input=MfsOriginalPipeWire::host_input;input.enabled=true;
        unsigned code=0;MaaKWinControlUnitHandle handle=nullptr;
        try {handle=MaaKWinControlUnitCreate("/capture-only-host",17,19,255);}
        catch(const KWinPortalFailure& error) {code=error.code;}
        // Preserve an unexpected source owner as failed evidence, never leak
        // it merely because the host premise/exception changed.
        if(handle) {MaaKWinControlUnitDestroy(handle);throw std::runtime_error("failed Portal created a manager");}
        const auto expected=stage==0?0u:90u+stage;
        const auto event_count=stage<2?2u:stage+1;
        const std::vector<std::string> order{"create","open","fd","node","destroy"};
        require(code==expected && kwin_host.helper && kwin_host.helper_ended==0
            && kwin_host.helper_closed==1 && kwin_host.events.size()==event_count
            && std::equal(kwin_host.events.begin(),kwin_host.events.end(),order.begin()),"original Portal failure/order differs");
        const int fd=kwin_host.helper->offered;
        const auto after=socket_cookie(fd);
        const auto residual_fds=count_entries("/proc/self/fd");
        require(fd>=0 && kwin_host.before!=0 && after==kwin_host.before
            && residual_fds==baseline_fds+1 && task_ids()==baseline
            && !input.native_constructed && input.creation_attempts==0,"source did not retain exactly the unconsumed helper socket");
        // Explicit fixture reclaim AFTER observing the source residual. Even
        // after a successful fd getter, no manager exists to consume the offer.
        const int reclaim=std::exchange(kwin_host.helper->offered,-1);
        require(::close(reclaim)==0,"fixture socket reclaim failed");
        errno=0;const bool closed=::fcntl(reclaim,F_GETFD)==-1 && errno==EBADF;
        kwin_host.helper.reset();
        const auto final_fds=count_entries("/proc/self/fd");
        const auto final_tasks=task_ids();
        save_tasks(directory/("closed-"+std::to_string(index)+"-task-ids.txt"),final_tasks);
        rows<<index<<'\t'<<stage<<'\t'<<code<<"\t1\t1\t0\t"<<fd<<'\t'<<kwin_host.before<<'\t'<<after
            <<'\t'<<unsigned(stage>=2)<<'\t'<<unsigned(stage>=3)<<'\t'<<unsigned(stage==4)<<"\t0\t"
            <<baseline_fds<<'\t'<<residual_fds<<'\t'<<final_fds<<'\t'<<baseline.size()<<'\t'<<::gettid()
            <<"\t1\t"<<closed<<"\t0\t0\t";
        for(size_t i=0;i<kwin_host.events.size();++i) {if(i) {rows<<',';}rows<<kwin_host.events[i];}rows<<'\n';rows.flush();
        require(closed && final_fds==baseline_fds && final_tasks==baseline,"fixture reclaim changed FD/task identities");
        input.enabled=false;
    }
    rows.close();
    std::cout<<"original KWin Portal failures cases=100 stages=5 residual_sockets=100 fixture_reclaims=100 manager_created=0 native_capture=0 full_P1_acceptance=0 passed\n";
}
}
