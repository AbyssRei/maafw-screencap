// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
namespace {
struct HeldAcquisition {
    IDXGIOutputDuplication* duplication;
    IDXGIResource* resource=nullptr;
    bool owned=false;
    explicit HeldAcquisition(IDXGIOutputDuplication* value):duplication(value){}
    HRESULT open(){
        DXGI_OUTDUPL_FRAME_INFO info{};
        IDXGIResource* raw=nullptr;
        const HRESULT code=duplication->AcquireNextFrame(2000,&info,&raw);
        owned=SUCCEEDED(code);
        if(owned)resource=raw;
        return code;
    }
    HRESULT close()noexcept{
        HRESULT code=E_PENDING;
        if(owned){owned=false;code=duplication->ReleaseFrame();}
        if(resource){resource->Release();resource=nullptr;}
        return code;
    }
    ~HeldAcquisition(){close();}
    HeldAcquisition(const HeldAcquisition&)=delete;
    HeldAcquisition& operator=(const HeldAcquisition&)=delete;
};
int held_acquisition(const std::filesystem::path& root){
    try{
        if(!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root))throw std::runtime_error("held-acquisition root must be empty");
        std::ofstream rows(root/"acquire.tsv",std::ios::binary),held_rows(root/"held.tsv",std::ios::binary),states(root/"state.tsv",std::ios::binary);
        rows.exceptions(std::ios::badbit|std::ios::failbit);held_rows.exceptions(std::ios::badbit|std::ios::failbit);states.exceptions(std::ios::badbit|std::ios::failbit);
        rows<<"cycle\tphase\toutcome\torder\tacquires\tcopies\tmaps\tunmaps\treleases\tconversions\tacquired\treleased\ttimeout\tpreserved\tmarker\tpitch\tthread\tcontext\tresource\tduplication\n";
        held_rows<<"cycle\tprepared\trejected\tmanual_release\tafter_release\tthread\tresource\tduplication\n";
        states<<"cycle\twidth\theight\tstaging\tduplication\tthread\tcopied_recovery\n";
        struct Saved{std::string name;cv::Mat image;};std::vector<Saved> saved;saved.reserve(40);
        for(uint32_t cycle=0;cycle<20;++cycle){
            Fixture fixture;
            if(mfs_print_thread_fixture(96,64,0,0,0,0,MFS_RENDER_DISABLE_TRANSITIONS|MFS_RENDER_TOPMOST,&fixture.value) || mfs_desktop_fixture_inside_updates(fixture.value.owner))throw std::runtime_error("held-acquisition fixture setup failed");
            {
                Source source(static_cast<HWND>(fixture.value.hwnd));
                auto warm=source.screencap();
                if(!warm || !source.warm())throw std::runtime_error("held-acquisition warm state failed");
                auto warm_target=target(*warm,static_cast<HWND>(fixture.value.hwnd));
                save(root,std::to_string(cycle)+"-warm",warm_target);saved.push_back({std::to_string(cycle)+"-warm-held",warm_target});
                auto* const duplication=source.duplication();auto* const staging=source.staging();
                const auto width=source.width(),height=source.height();
                HeldAcquisition held(duplication);const HRESULT prepared=held.open();
                if(prepared!=S_OK || !held.resource)throw std::runtime_error("held-acquisition native precondition failed");
                const auto held_resource=reinterpret_cast<uintptr_t>(held.resource);
                mfs_dup_map::Trace rejection;
                std::optional<cv::Mat> rejected;
                {mfs_dup_map::Scope scope(rejection);rejected=source.screencap();}
                const bool preserved=source.warm() && source.duplication()==duplication && source.staging()==staging && source.width()==width && source.height()==height;
                if(rejected || !preserved || rejection.violation || rejection.held || rejection.mapped || rejection.order!=1
                    || rejection.acquires!=1 || rejection.copies || rejection.maps || rejection.unmaps || rejection.releases || rejection.conversions
                    || rejection.acquired!=DXGI_ERROR_INVALID_CALL || rejection.timeout!=2000 || rejection.context || rejection.resource
                    || rejection.duplication!=reinterpret_cast<uintptr_t>(duplication) || rejection.thread!=GetCurrentThreadId())throw std::runtime_error("original held-acquisition rejection/state differs");
                // This S_OK is a native witness that the failed capture did not
                // release the caller's preexisting frame. Neither call is a retry.
                const HRESULT released=held.close();
                const HRESULT after_release=duplication->ReleaseFrame();
                held_rows<<cycle<<'\t'<<prepared<<'\t'<<rejection.acquired<<'\t'<<released<<'\t'<<after_release<<'\t'<<GetCurrentThreadId()<<'\t'<<held_resource<<'\t'<<reinterpret_cast<uintptr_t>(duplication)<<'\n';held_rows.flush();
                if(released!=S_OK || after_release!=DXGI_ERROR_INVALID_CALL)throw std::runtime_error("original caller-held native cleanup differs");
                rows<<cycle<<"\trejected\t0\t1\t1\t0\t0\t0\t0\t0\t"<<rejection.acquired<<"\t0\t2000\t1\t-1\t0\t"<<rejection.thread<<"\t0\t0\t"<<rejection.duplication<<'\n';rows.flush();
                mfs_dup_map::Trace recovery;std::optional<cv::Mat> image;
                {mfs_dup_map::Scope scope(recovery);image=source.screencap();}
                if(!image || !source.warm() || source.duplication()!=duplication || source.staging()!=staging || source.width()!=width || source.height()!=height
                    || recovery.violation || recovery.held || recovery.mapped || recovery.order!=123456
                    || recovery.acquires!=1 || recovery.copies!=1 || recovery.maps!=1 || recovery.unmaps!=1 || recovery.releases!=1 || recovery.conversions!=1
                    || recovery.acquired!=S_OK || recovery.released!=S_OK || recovery.mapped_result!=S_OK || recovery.subresource || recovery.timeout!=2000
                    || recovery.thread!=GetCurrentThreadId() || !recovery.context || recovery.resource!=reinterpret_cast<uintptr_t>(staging) || recovery.duplication!=reinterpret_cast<uintptr_t>(duplication))throw std::runtime_error("original single recovery differs");
                auto roi=target(*image,static_cast<HWND>(fixture.value.hwnd));save(root,std::to_string(cycle)+"-recovery",roi);const int marker=known(roi);
                saved.push_back({std::to_string(cycle)+"-recovery-held",roi});
                rows<<cycle<<"\trecovery\t1\t123456\t1\t1\t1\t1\t1\t1\t"<<recovery.acquired<<'\t'<<recovery.released<<"\t2000\t1\t"<<marker<<'\t'<<recovery.pitch<<'\t'<<recovery.thread<<'\t'<<recovery.context<<'\t'<<recovery.resource<<'\t'<<recovery.duplication<<'\n';rows.flush();
                states<<cycle<<'\t'<<width<<'\t'<<height<<'\t'<<reinterpret_cast<uintptr_t>(staging)<<'\t'<<reinterpret_cast<uintptr_t>(duplication)<<'\t'<<GetCurrentThreadId()<<"\t-1\n";states.flush();
            }
            fixture.close();
        }
        for(const auto& value:saved)save(root,value.name,value.image);
        if(saved.size()!=40)throw std::runtime_error("original held-acquisition matrix incomplete");
        std::cout<<"original DesktopDup held_acquisition cycles=20 public_calls=60 observed_calls=40 native_rejections=20 caller_acquires=20 caller_releases=20 recovery=20 retained=40 full_P1_acceptance=0 passed\n";
        return 0;
    }catch(const std::exception& error){std::cerr<<"held-acquisition failure="<<error.what()<<'\n';return 1;}
}
}
