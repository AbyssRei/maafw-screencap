// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "Screencap/WlrScreencap.h"
#include "Screencap/NoneScreencap.h"
#include "MaaUtils/Logger.h"
#include <opencv2/imgproc.hpp>
#include <algorithm>
#include <array>
#include <cerrno>
#include <dlfcn.h>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <limits>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <sys/random.h>
#include <sys/socket.h>
#include <unistd.h>
#include <vector>
namespace {
void require(bool value,const char* message) {if(!value) {throw std::runtime_error(message);}}
size_t fds() {return static_cast<size_t>(std::distance(std::filesystem::directory_iterator("/proc/self/fd"),std::filesystem::directory_iterator{}));}
std::vector<std::string> tasks() {
    std::vector<std::string> ids;
    for(const auto& entry:std::filesystem::directory_iterator("/proc/self/task")) {ids.push_back(entry.path().filename().string());}
    std::sort(ids.begin(),ids.end());return ids;
}
void text(const std::filesystem::path& path,std::string_view value) {
    require(value.size()<=static_cast<size_t>(std::numeric_limits<std::streamsize>::max()),"record span overflow");
    std::ofstream out(path,std::ios::binary);out.write(value.data(),static_cast<std::streamsize>(value.size()));
    out.close();require(out.good(),"cannot save Wlr native record");
}
void task_record(const std::filesystem::path& path,const std::vector<std::string>& ids) {
    std::string value;for(const auto& id:ids) {value+=id+'\n';}text(path,value);
}
void pixels(const cv::Mat& image) {
    require(image.type()==CV_8UC3 && image.cols==64 && image.rows==48,"Wlr native layout differs");
    for(int y=0;y<48;++y) {for(int x=0;x<64;++x) {
        const cv::Vec3b expected=y>=24?cv::Vec3b(255,0,0):x<32?cv::Vec3b(0,0,255):cv::Vec3b(0,255,0);
        require(image.at<cv::Vec3b>(y,x)==expected,"Wlr native known pixel differs");
    }}
}
void save(const std::filesystem::path& path,const cv::Mat& image) {
    pixels(image);std::ofstream out(path,std::ios::binary);
    for(int row=0;row<48;++row) {out.write(reinterpret_cast<const char*>(image.ptr(row)),192);}
    out.close();require(out.good(),"cannot save Wlr native pixels");
}
}
#include "native-owner.hpp"
MAA_CTRL_UNIT_NS_BEGIN
#include "uuid-context.inc"
MAA_CTRL_UNIT_NS_END
int main(int argc,char** argv) {
    try {
        std::cout.setf(std::ios::unitbuf);
        const bool construction=argc==5 && std::string_view(argv[4])=="--host-create-failure";
        require(argc==4 || construction,"usage: native-context <private socket> <new artifact directory> <actual libwayland> [--host-create-failure]");
        Dl_info owner{};
        require(dladdr(reinterpret_cast<void*>(&wl_display_roundtrip),&owner)!=0 && owner.dli_fname
            && std::filesystem::canonical(owner.dli_fname)==std::filesystem::canonical(argv[3]),"wrong native Wayland library");
        const std::filesystem::path directory=argv[2];require(std::filesystem::create_directory(directory),"Wlr native records already exist");
        require(std::string_view(MaaWlRootsControlUnitGetVersion())=="mfs-p1-wlr-native","wrong actual compat version");
        MaaWlRootsControlUnitDestroy(nullptr);
        text(directory/"socket-path.txt",argv[1]);
        cv::Mat warm(48,64,CV_8UC4,cv::Scalar(0,0,255,255)),converted;cv::cvtColor(warm,converted,cv::COLOR_BGRA2BGR);
        std::ofstream cases(directory/"source-cases.tsv");require(cases.good(),"cannot create native cases");
        const auto baseline_fds=fds();const auto baseline_tasks=tasks();task_record(directory/"baseline-task-ids.txt",baseline_tasks);
        std::vector<cv::Mat> held;held.reserve(60);
        for(unsigned cycle=0;cycle<20;++cycle) {
            MAA_CTRL_UNIT_NS::host={};auto& host=MAA_CTRL_UNIT_NS::host;host.outcome=construction?3:cycle%3;
            int result=2,code=0;uint64_t features=0;long after_refs=0;unsigned captures=0;
            {
                std::unique_ptr<MAA_CTRL_UNIT_NS::LinuxControlUnitAPI,decltype(&MaaWlRootsControlUnitDestroy)> unit(
                    MaaWlRootsControlUnitCreate(argv[1],255),MaaWlRootsControlUnitDestroy);
                require(unit && dynamic_cast<MAA_CTRL_UNIT_NS::ScrollableUnit*>(unit.get())
                    && dynamic_cast<MAA_CTRL_UNIT_NS::RelativeMovableUnit*>(unit.get()),"original public handle/bases missing");
                std::string uuid;require(unit->request_uuid(uuid) && uuid==std::string(argv[1])+"_"+argv[1],"Wlr UUID lost original input");
                const auto metadata=json::value(unit->get_info()).to_string();
                text(directory/("api-"+std::to_string(cycle)+".uuid"),uuid);text(directory/("api-"+std::to_string(cycle)+".info"),metadata);
                cv::Mat output;require(!unit->connected() && unit->get_features()==0 && !unit->screencap(output) && output.empty(),"unconnected Wlr virtual state differs");
                try {result=unit->connect()?1:0;}catch(const MAA_CTRL_UNIT_NS::HostInitFailure&) {code=97;}
                catch(const MAA_CTRL_UNIT_NS::HostCreateFailure&) {code=98;}
                if(construction) {
                    require(result==2 && code==98 && !unit->connected() && unit->get_features()==0,"Wlr constructor failure state differs");
                    const auto client=host.client.lock();require(bool(client),"partial capture lost its client");
                    after_refs=client.use_count()-1;host.connected=client->connected();
                    require(after_refs==2 && host.connected==1,"constructor error lost manager/capture client owners");
                }else {
                    require(result==(host.outcome==0?1:host.outcome==1?0:2) && code==(host.outcome==2?97:0)
                        && unit->connected()==(host.outcome==0) && unit->get_features()==3,"Wlr host init/feature state differs");
                }
                for(unsigned call=0;call<3;++call) {
                    if(call==1) {require(unit->inactive(),"Wlr virtual inactive failed");}
                    require(unit->screencap(output),"Wlr virtual capture after host init failed");pixels(output);
                    ++captures;
                    save(directory/("host-"+std::to_string(cycle)+"-call-"+std::to_string(call)+".bgr"),output);
                    held.push_back(std::move(output));for(const auto& image:held) {pixels(image);}
                }
                require(json::value(unit->get_info()).to_string()==metadata,"Wlr metadata changed after capture/inactive");features=unit->get_features();
            }
            const bool expired=host.client.expired();
            const bool snapshots=std::all_of(host.snapshots.begin(),host.snapshots.end(),[](const auto& weak){return weak.expired();});
            const auto final_fds=fds();const auto final_tasks=tasks();
            task_record(directory/("closed-"+std::to_string(cycle)+"-task-ids.txt"),final_tasks);
            // Shared format: see xtask/wlr_host_api.rs. Scalar evidence is
            // written before assertions so a failed lifetime remains reviewable.
            if(construction) {
                cases<<cycle<<'\t'<<code<<'\t'<<host.attempts<<'\t'<<host.created<<'\t'<<host.initialized<<'\t'<<host.inactive<<'\t'<<host.destroyed
                    <<'\t'<<host.create_refs<<'\t'<<after_refs<<'\t'<<host.connected<<'\t'<<captures<<'\t'<<features<<"\t0\t"<<expired
                    <<'\t'<<baseline_fds<<'\t'<<final_fds<<'\t'<<baseline_tasks.size()<<'\t'<<(final_tasks==baseline_tasks)
                    <<'\t'<<host.snapshot_mask<<'\t'<<snapshots<<'\n';cases.flush();
                require(cases.good() && host.attempts==1 && host.created==0 && host.initialized==0 && host.inactive==0 && host.destroyed==0
                    && host.create_refs==3 && after_refs==2 && host.connected==1 && host.closed==0 && host.errors==0 && captures==3
                    && features==0 && expired && snapshots && host.snapshot_mask==29 && baseline_fds==final_fds && final_tasks==baseline_tasks,
                    "Wlr constructor-error retained capture/resources differ");
                continue;
            }
            cases<<cycle<<'\t'<<host.outcome<<'\t'<<result<<'\t'<<code<<'\t'<<host.created<<'\t'<<host.initialized<<'\t'<<host.inactive<<'\t'<<host.destroyed
                <<'\t'<<host.create_refs<<'\t'<<host.destroy_refs<<'\t'<<host.connected<<'\t'<<host.closed<<'\t'<<host.errors<<'\t'<<expired
                <<'\t'<<baseline_fds<<'\t'<<final_fds<<'\t'<<baseline_tasks.size()<<'\t'<<(final_tasks==baseline_tasks)<<'\t'<<features<<"\t1\t1\t3\t1\t1\n";cases.flush();
            text(directory/("snapshots-"+std::to_string(cycle)+".txt"),std::to_string(host.snapshot_mask)+"\t"+(snapshots?"1\n":"0\n"));
            require(cases.good() && host.created==1 && host.initialized==1 && host.inactive==1 && host.destroyed==1 && host.create_refs==3
                && host.destroy_refs==1 && host.connected==1 && host.closed==2 && host.errors==0 && expired && snapshots && host.snapshot_mask==29
                && baseline_fds==final_fds && final_tasks==baseline_tasks,"Wlr shared input/client destruction differs");
        }
        for(size_t index=0;index<held.size();++index) {save(directory/("held-"+std::to_string(index)+".bgr"),held[index]);}
        if(construction) {
            std::cout<<"original Wlr host_create_failure objects=20 constructor_errors=20 captures=60 retained=60 real_virtual_handles=20 clients_released=20 resources_balanced=1 input_actions=0 passed\n";
        }else {
            std::cout<<"original Wlr ABI objects=20 host_true_false_error=7,7,6 captures=60 retained=60 exact_compat_create_destroy=20 real_virtual_handles=20 shared_last_client=20 final_client_snapshots_expired=20 input_actions=0 passed\n";
        }
        return 0;
    }catch(const std::exception& error) {std::cerr<<"Wlr native ABI failure: "<<error.what()<<'\n';return 1;}
}
