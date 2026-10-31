// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "Screencap/WlrScreencap.h"
#include "Screencap/NoneScreencap.h"
#include "MaaFramework/MaaDef.h"
#include "MaaUtils/Logger.h"
#include <opencv2/imgproc.hpp>
#include <array>
#include <algorithm>
#include <cerrno>
#include <cstring>
#include <dlfcn.h>
#include <exception>
#include <fcntl.h>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <sys/random.h>
#include <sys/mman.h>
#include <sys/un.h>
#include <thread>
#include <utility>
#include <unistd.h>
#include <vector>
#include "protocol-pattern.h"

extern "C" void __real_wl_display_disconnect(wl_display*);

namespace MfsOriginalWlr {
#define MFS_MANAGER_WLR
#include "../linux-manager/native-shim.h"
#undef MFS_MANAGER_WLR
}

namespace {
uint64_t roundtrips=0;
uint32_t depth=0,maximum_depth=0;
bool counter_error=false;
void require(bool condition,const char* message) {
    if(!condition) { throw std::runtime_error(message); }
}
size_t count(const char* directory) {
    return static_cast<size_t>(std::distance(std::filesystem::directory_iterator(directory),
        std::filesystem::directory_iterator{}));
}
std::vector<std::string> tasks() {
    std::vector<std::string> result;
    for(const auto& entry:std::filesystem::directory_iterator("/proc/self/task")) {
        result.push_back(entry.path().filename().string());
    }
    std::sort(result.begin(),result.end());
    return result;
}
void pixels(const cv::Mat& image) {
    require(image.type()==CV_8UC3 && image.cols==64 && image.rows==48,
        "original Wlr image layout mismatch");
    for(int y=0;y<48;++y) { for(int x=0;x<64;++x) {
        const auto actual=image.at<cv::Vec3b>(y,x);
        const cv::Vec3b expected=y>=24 ? cv::Vec3b(255,0,0) :
            x<32 ? cv::Vec3b(0,0,255) : cv::Vec3b(0,255,0);
        require(actual==expected,"original Wlr known pixel mismatch");
    } }
}
void save(const cv::Mat& image,const std::filesystem::path& path) {
    std::ofstream file(path,std::ios::binary);
    require(image.type()==CV_8UC3 && image.cols==64 && image.rows==48,
        "cannot serialize unexpected Wlr image");
    for(int y=0;y<48;++y) { file.write(reinterpret_cast<const char*>(image.ptr(y)),64*3); }
    file.close();require(file.good(),"cannot save Wlr original image");
}
#include "protocol-context.hpp"
#include "registry-context.hpp"
#include "failure-context.hpp"
#include "early-close-context.hpp"
void shared_client(const std::filesystem::path& socket,const std::filesystem::path& directory) {
    const auto baseline_fds=count("/proc/self/fd");
    const auto baseline_tasks=tasks();
    std::vector<cv::Mat> held;held.reserve(60);
    for(int cycle=0;cycle<20;++cycle) {
        auto client=std::make_shared<MfsOriginalWlr::WaylandClient>();
        require(client->connect(socket),"original shared client connect failed");
        std::weak_ptr<MfsOriginalWlr::WaylandClient> owner=client;
        auto first=std::make_unique<MfsOriginalWlr::WlrScreencap>(client);
        auto second=std::make_unique<MfsOriginalWlr::WlrScreencap>(client);
        client.reset();
        require(!owner.expired(),"source did not retain shared client");
        for(int call=0;call<3;++call) {
            if(call==2) {
                first.reset();
                require(!owner.expired(),"first source destruction closed second source client");
            }
            cv::Mat image;
            auto& source=call==0 ? *first : *second;
            require(source.screencap(image),"original shared source capture failed");
            pixels(image);
            save(image,directory/("shared-"+std::to_string(cycle)+"-"+std::to_string(call)+".bgr"));
            held.push_back(std::move(image));
        }
        second.reset();
        require(owner.expired(),"last source did not release shared client");
        const auto actual_fds=count("/proc/self/fd");
        require(actual_fds==baseline_fds && tasks()==baseline_tasks,
            "original shared client resources did not return to baseline");
        std::cout<<"original shared cycle="<<cycle<<" held="<<held.size()
            <<" fds="<<actual_fds<<" baseline_fds="<<baseline_fds<<'\n';
    }
    std::exception_ptr error;
    std::thread reader([&] { try { for(const auto& image:held) { pixels(image); } }
        catch(...) { error=std::current_exception(); } });
    reader.join();if(error) { std::rethrow_exception(error); }
    std::cout<<"original Wlr shared_client cycles=20 captures=60 caller_released=20 first_source_released=20 last_client_released=20 retained=60 resources_balanced=1 passed\n";
}
void manager_context(const std::filesystem::path& socket,const std::filesystem::path& directory) {
    const auto baseline_fds=count("/proc/self/fd");
    const auto baseline_tasks=tasks();
    std::vector<cv::Mat> held;held.reserve(60);
    for(int cycle=0;cycle<20;++cycle) {
        std::weak_ptr<MfsOriginalWlr::WaylandClient> owner;
        {
            MfsOriginalWlr::LinuxControlUnitConfig config;
            config.screencap_method=MaaLinuxScreencapMethod_Wlr;config.wlr_socket_path=socket.string();
            MfsOriginalWlr::LinuxControlUnitMgr manager(config);
            require(manager.connect() && manager.connected(),"original Wlr manager initial connect failed");
            owner=manager.wl_client_;
            const auto* output=manager.wl_client_->get_output().get();
            for(int call=0;call<3;++call) {
                if(call==1) { require(manager.inactive(),"original Wlr manager inactive failed"); }
                if(call==2) {
                    const auto before=roundtrips;
                    require(manager.connect() && manager.connected(),"original Wlr manager reconnect failed");
                    require(roundtrips==before && manager.wl_client_->get_output().get()==output,
                        "original manager unexpectedly reopened its retained client");
                }
                cv::Mat image;require(manager.screencap(image),"original Wlr manager forwarding failed");
                pixels(image);save(image,directory/("manager-"+std::to_string(cycle)+"-"+std::to_string(call)+".bgr"));
                held.push_back(std::move(image));
            }
        }
        require(owner.expired(),"original Wlr manager left a live client owner");
        require(count("/proc/self/fd")==baseline_fds && tasks()==baseline_tasks,"original Wlr manager resource mismatch");
        std::cout<<"original Wlr manager cycle="<<cycle<<" held="<<held.size()<<" client_released=1 fds="<<baseline_fds<<'\n';
    }
    const auto missing=socket.parent_path()/"mfs-manager-missing";
    require(!std::filesystem::exists(missing),"manager missing socket fixture exists");
    {
        MfsOriginalWlr::LinuxControlUnitConfig config;
        config.screencap_method=MaaLinuxScreencapMethod_Wlr;config.wlr_socket_path=missing.string();
        MfsOriginalWlr::LinuxControlUnitMgr manager(config);
        require(!manager.connect() && !manager.connected(),"original failed client did not remain disconnected");
        cv::Mat output=held.back();const auto* data=output.data;const int owners=output.u->refcount;
        require(!manager.screencap(output) && output.data==data && output.u->refcount==owners,"original pre-reconnect failure changed output");
        require(manager.connect() && manager.connected(),"original retained failed client connect state changed");
        require(!manager.screencap(output) && output.data==data && output.u->refcount==owners,"original disconnected client changed output");
        pixels(output);
    }
    require(count("/proc/self/fd")==baseline_fds && tasks()==baseline_tasks,"original failed manager resource mismatch");
    std::exception_ptr error;
    std::thread reader([&]{try {for(const auto& image:held) {pixels(image);} }catch(...) {error=std::current_exception();}});
    reader.join();if(error) {std::rethrow_exception(error);}
    std::cout<<"original Wlr manager cycles=20 captures=60 connects=40 retained=60 same_client_reconnect=20 failed_client_retained=1 failed_output_preserved=1 resources_balanced=1 passed\n";
}
void reconnect_client(const std::filesystem::path& socket,const std::filesystem::path& directory) {
    const auto alias=socket.parent_path()/"mfs-source-alias";
    const auto missing=socket.parent_path()/"mfs-source-missing";
    require(!std::filesystem::exists(alias) && !std::filesystem::exists(missing),"source reconnect fixture path already exists");
    std::filesystem::create_symlink(socket,alias);
    struct Alias {
        std::filesystem::path path;
        ~Alias() { std::error_code ignored;std::filesystem::remove(path,ignored); }
    } cleanup_alias{alias};
    const auto baseline_fds=count("/proc/self/fd");
    const auto baseline_tasks=tasks();
    for(int cycle=0;cycle<20;++cycle) {
        auto client=std::make_shared<MfsOriginalWlr::WaylandClient>();
        require(client->connect(socket),"original initial connect failed");
        const auto* output=client->get_output().get();
        const auto before=roundtrips;
        require(client->connect(socket) && client->get_output().get()==output && roundtrips==before,
            "same-path connect unexpectedly reopened original client");
        client->close();client->close();
        require(!client->connected(),"original repeated close left connected state");
        require(client->connect(socket) && roundtrips>before,"original same-path reopen failed");
        const auto before_change=roundtrips;
        require(client->connect(alias) && roundtrips>before_change,"changed-path original reconnect failed");
        {
            MfsOriginalWlr::WlrScreencap source(client);
            cv::Mat image;require(source.screencap(image),"changed-path original capture failed");
            pixels(image);save(image,directory/("reconnected-"+std::to_string(cycle)+".bgr"));
        }
        // No capture or external proxy owner survives these close/reopen calls.
        require(!client->connect(missing) && !client->connected() && !client->get_output(),
            "failed changed-path original connect did not clear old client");
        require(client->connect(socket),"original connect after missing path failed");
        client->close();client.reset();
        const auto actual_fds=count("/proc/self/fd");
        require(actual_fds==baseline_fds && tasks()==baseline_tasks,
            "original reconnect resources did not return to baseline");
        std::cout<<"original reconnect cycle="<<cycle<<" fds="<<actual_fds
            <<" baseline_fds="<<baseline_fds<<" roundtrips="<<roundtrips<<'\n';
    }
    std::cout<<"original Wlr reconnect cycles=20 same_path_noop=20 close_reopen=20 changed_path=20 missing_path=20 subsequent_connect=20 captures=20 resources_balanced=1 passed\n";
}
#include "name-context.hpp"
void fd_zero(const std::filesystem::path& directory) {
    constexpr int size=12288;
    const auto baseline_fds=count("/proc/self/fd");
    const int saved=fcntl(STDIN_FILENO,F_DUPFD_CLOEXEC,3);
    require(saved>=3,"cannot preserve isolated fixture stdin");
    require(close(STDIN_FILENO)==0,"cannot free isolated fixture FD zero");
    void* mapping=nullptr;
    {
        MfsOriginalWlr::MemfdBuffer buffer(size);
        require(buffer.fd()==0 && buffer.available() && buffer.ptr()!=nullptr,
            "original MemfdBuffer did not successfully own FD zero");
        mapping=buffer.ptr();
        static_cast<unsigned char*>(mapping)[0]=0xa5;
    }
    const int remaining=fcntl(STDIN_FILENO,F_GETFD);
    const auto page=sysconf(_SC_PAGESIZE);require(page>0,"cannot query native page size");
    std::vector<unsigned char> resident((static_cast<size_t>(size)+static_cast<size_t>(page)-1)/static_cast<size_t>(page));
    const int mapped=mincore(mapping,size,resident.data());
    require(remaining>=0 && mapped==0,"fixed-source FD-zero destructor residual was not observed");
    require(static_cast<unsigned char*>(mapping)[0]==0xa5,"original residual mapping contents changed");
    // Explicit fixture recovery AFTER observing the unchanged destructor. These
    // calls are not source cleanup success and never execute in capture code.
    require(munmap(mapping,size)==0 && close(STDIN_FILENO)==0,"fixture residual reclaim failed");
    require(dup2(saved,STDIN_FILENO)==STDIN_FILENO && close(saved)==0,"fixture stdin restoration failed");
    require(count("/proc/self/fd")==baseline_fds,"fixture recovery left an extra descriptor");
    std::ofstream evidence(directory/"fd-zero.txt");
    evidence<<"source_available=1\nsource_fd=0\nsource_destructor_fd_retained=1\nsource_destructor_mapping_retained=1\nfixture_reclaimed=1\nproduct_cleanup_success=0\n";
    evidence.close();require(evidence.good(),"cannot save original FD-zero evidence");
    std::cout<<"original MemfdBuffer fd_zero=1 available=1 destructor_fd_retained=1 destructor_mapping_retained=1 fixture_reclaimed=1 product_cleanup_success=0 passed\n";
}
}

extern "C" int __real_wl_display_roundtrip(wl_display* display);
extern "C" int __wrap_wl_display_roundtrip(wl_display* display) {
    // All calls belong to this process's sole Wayland channel. No allocation,
    // exception or logging is introduced into the original recursive call.
    if(roundtrips==UINT64_MAX || depth==UINT32_MAX) { counter_error=true; }
    else { ++roundtrips;++depth;maximum_depth=std::max(maximum_depth,depth); }
    const int result=__real_wl_display_roundtrip(display);
    if(depth>0) { --depth; }
    return result;
}

namespace MfsOriginalWlr {
#include "uuid-context.inc"
}

extern "C" void __wrap_wl_display_disconnect(wl_display* display) noexcept {
    // Normal modes forward immediately. Only the explicitly named prefix
    // probe defers an unsafe disconnect until retained proxy owners end.
    if(close_observer) {close_observer->observe(display);}
    else {__real_wl_display_disconnect(display);}
}

int main(int argc,char** argv) {
    try {
        std::cout.setf(std::ios::unitbuf);
        require(argc==4 || argc==5,"usage: source-context <private socket> <artifact dir> <libwayland path> [shared|reconnect|early-close-prefix|fd-zero|manager|nul-names|path-names|registry|failures|protocol-1|protocol-2|protocol-3]");
        Dl_info owner{};
        // The linked wrapper has its own address; inspect the real target.
        require(dladdr(reinterpret_cast<void*>(&__real_wl_display_roundtrip),&owner)!=0 && owner.dli_fname,
            "cannot identify actual native roundtrip target");
        require(std::filesystem::canonical(owner.dli_fname)==std::filesystem::canonical(argv[3]),
            "original Wlr loaded a different libwayland-client");
        Dl_info disconnect_owner{};
        require(dladdr(reinterpret_cast<void*>(&__real_wl_display_disconnect),&disconnect_owner)!=0 && disconnect_owner.dli_fname
            && std::filesystem::canonical(disconnect_owner.dli_fname)==std::filesystem::canonical(argv[3]),
            "original Wlr disconnect uses a different libwayland-client");
        std::cout<<"native roundtrip owner="<<owner.dli_fname<<'\n';
        const std::filesystem::path directory=argv[2];
        std::filesystem::create_directories(directory);
        // Initialize only conversion runtime before native resource baselines.
        cv::Mat warm(48,64,CV_8UC4,cv::Scalar(0,0,255,255)),converted;
        cv::cvtColor(warm,converted,cv::COLOR_BGRA2BGR);
        if(argc==5) {
            const std::string mode=argv[4];
            if(mode=="shared") { shared_client(argv[1],directory); }
            else if(mode=="manager") { manager_context(argv[1],directory); }
            else if(mode=="reconnect") { reconnect_client(argv[1],directory); }
            else if(mode=="early-close-prefix") { early_close_context(argv[1],directory); }
            else if(mode=="fd-zero") { fd_zero(directory); }
            else if(mode=="nul-names") { nul_names(argv[1],directory); }
            else if(mode=="path-names") { path_names(argv[1],directory); }
            else if(mode=="registry") { registry_context(argv[1],directory); }
            else if(mode=="failures") { failure_context(argv[1],directory); }
            else if(mode=="protocol-1" || mode=="protocol-2" || mode=="protocol-3") {
                protocol_context(argv[1],directory,static_cast<unsigned>(mode.back()-'0'));
            }
            else { throw std::runtime_error("unknown original Wlr context mode"); }
            return 0;
        }
        const auto fds=count("/proc/self/fd");
        const auto baseline_tasks=tasks();
        std::vector<cv::Mat> held;held.reserve(60);
        for(int cycle=0;cycle<20;++cycle) {
            const auto before=roundtrips;
            {
                auto client=std::make_shared<MfsOriginalWlr::WaylandClient>();
                require(client->connect(argv[1]) && client->connected(),"original Wlr connect failed");
                {
                    MfsOriginalWlr::WlrScreencap source(client);
                    require(source.init(),"original inherited Wlr init failed");
                    for(int call=0;call<3;++call) {
                        if(call==1) { source.inactive(); }
                        cv::Mat image;
                        require(source.screencap(image),"original Wlr capture failed");
                        save(image,directory/("cycle-"+std::to_string(cycle)+"-call-"+std::to_string(call)+".bgr"));
                        pixels(image);held.push_back(std::move(image));
                    }
                }
                client->close();client->close();
                require(!client->connected(),"closed original Wlr client remained connected");
            }
            const auto actual_fds=count("/proc/self/fd");
            const auto actual_tasks=tasks();
            std::cout<<"original Wlr cycle="<<cycle<<" captures=3 retained="<<held.size()
                <<" fds="<<actual_fds<<" baseline_fds="<<fds<<" roundtrips="<<roundtrips-before
                <<" max_depth="<<maximum_depth<<'\n';
            require(actual_fds==fds && actual_tasks==baseline_tasks,"original Wlr native resource mismatch");
            require(!counter_error && depth==0 && maximum_depth>=2,
                "original Wlr recursive roundtrip was not observed");
            for(const auto& image:held) { pixels(image); }
        }
        std::exception_ptr error;
        std::thread reader([&] { try { for(const auto& image:held) { pixels(image); } }
            catch(...) { error=std::current_exception(); } });
        reader.join();if(error) { std::rethrow_exception(error); }
        std::ofstream evidence(directory/"roundtrips.txt");
        evidence<<"calls="<<roundtrips<<" maximum_depth="<<maximum_depth<<" final_depth="<<depth<<'\n';
        evidence.close();require(evidence.good(),"cannot save roundtrip observations");
        std::cout<<"original Wlr source cycles=20 captures=60 retained=60 recursive_roundtrip=1 resources_balanced=1 passed\n";
        return 0;
    }catch(const std::exception& error) { std::cerr<<"Wlr source context failure: "<<error.what()<<'\n';return 1; }
    catch(...) { std::cerr<<"Wlr source context unknown exception\n";return 2; }
}
