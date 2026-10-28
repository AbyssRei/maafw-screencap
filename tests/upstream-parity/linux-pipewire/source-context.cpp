// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "Screencap/PipeWireScreencap.h"
#include <pipewire/pipewire.h>
#include <opencv2/imgproc.hpp>
#include <array>
#include <cerrno>
#include <charconv>
#include <chrono>
#include <cstring>
#include <cstdlib>
#include <exception>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string_view>
#include <thread>
#include <vector>
#include <fcntl.h>
#include <dlfcn.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

namespace {
using Source=MfsOriginalPipeWire::PipeWireScreencap;
constexpr int WIDTH=28,HEIGHT=24;
void require(bool condition,const char* message) {
    if(!condition) { throw std::runtime_error(message); }
}
void verify_runtime() {
    const char* version=std::getenv("MFS_P0_PIPEWIRE_VERSION");
    const char* library=std::getenv("MFS_P1_PIPEWIRE_LIBRARY");
    require(version && library,"explicit native version/library identities are required");
    std::cout<<"source headers="<<pw_get_headers_version()<<" runtime="<<pw_get_library_version()<<" expected="<<version<<'\n';
    require(std::string_view(pw_get_headers_version())==version && std::string_view(pw_get_library_version())==version,"source headers/runtime differ from the selected PipeWire version");
    Dl_info info{};
    void* symbol=dlsym(RTLD_DEFAULT,"pw_init");
    require(symbol && dladdr(symbol,&info)!=0 && info.dli_fname,"cannot identify native PipeWire symbol owner");
    std::cout<<"source symbol_owner="<<info.dli_fname<<" expected_library="<<library<<'\n';
    require(std::filesystem::canonical(info.dli_fname)==std::filesystem::canonical(library),"source loaded a different PipeWire library");
    std::cout<<"source runtime="<<version<<" library="<<info.dli_fname<<'\n';
}
uint32_t known_pixels(const cv::Mat& image) {
    require(image.type()==CV_8UC3 && image.cols==WIDTH && image.rows==HEIGHT,"source output layout mismatch");
    const auto header=image.at<cv::Vec3b>(0,0);
    const uint32_t marker=(static_cast<uint32_t>(header[2])<<16)|(static_cast<uint32_t>(header[1])<<8)|header[0];
    require(marker!=0,"source fixture marker is absent");
    constexpr std::array<std::array<uint8_t,3>,7> rgb={{{191,191,191},{191,191,0},{0,191,191},{0,191,0},{191,0,191},{191,0,0},{0,0,191}}};
    for(int y=0;y<HEIGHT;++y) {
        for(int x=0;x<WIDTH;++x) {
            if(x==0 && y==0) { continue; } // producer serial in RGB byte order
            const auto& base=rgb[static_cast<size_t>(x)/4];
            std::array<uint8_t,3> expected{};
            for(size_t channel=0;channel<3;++channel) {
                expected[channel]=static_cast<uint8_t>(base[channel] ^ ((static_cast<size_t>(y)*7+static_cast<size_t>(x)*3+channel*13+(marker&63))&63));
            }
            const auto pixel=image.at<cv::Vec3b>(y,x);
            require(pixel[0]==expected[2] && pixel[1]==expected[1] && pixel[2]==expected[0],"source known-color mismatch");
        }
    }
    return marker;
}
struct HeldImage { cv::Mat image;uint32_t marker; };
size_t count_entries(const char* path) {
    size_t count=0;
    for([[maybe_unused]] const auto& entry:std::filesystem::directory_iterator(path)) { ++count; }
    return count;
}
int connect_socket(const char* path) {
    sockaddr_un address{};
    address.sun_family=AF_UNIX;
    const size_t length=std::strlen(path);
    require(length>0 && length<sizeof(address.sun_path),"private socket path does not fit sockaddr_un");
    std::memcpy(address.sun_path,path,length+1);
    const int fd=socket(AF_UNIX,SOCK_STREAM|SOCK_CLOEXEC,0);
    require(fd>=0,"source socket creation failed");
    if(connect(fd,reinterpret_cast<const sockaddr*>(&address),sizeof(address))!=0) {
        const int saved=errno;
        close(fd);
        throw std::runtime_error(std::string("private source socket connect failed: ")+std::strerror(saved));
    }
    return fd;
}
uint64_t socket_cookie(int fd) {
    uint64_t cookie=0;
    socklen_t length=sizeof(cookie);
    require(getsockopt(fd,SOL_SOCKET,SO_COOKIE,&cookie,&length)==0 && length==sizeof(cookie),"cannot establish owned socket identity");
    return cookie;
}
struct ProbeInit {
    ProbeInit() { pw_init(nullptr,nullptr); }
    ~ProbeInit() { pw_deinit(); }
};
struct ProbeGraph {
    pw_main_loop* loop=nullptr;
    pw_context* context=nullptr;
    pw_core* core=nullptr;
    ~ProbeGraph() {
        if(core) { pw_core_disconnect(core); }
        if(context) { pw_context_destroy(context); }
        if(loop) { pw_main_loop_destroy(loop); }
    }
};
struct ProbeSocket {
    int fd;
    uint64_t cookie=0;
    bool offered=false;
    ~ProbeSocket() {
        if(fd<0) { return; }
        if(!offered) { close(fd);return; }
        uint64_t actual=0;
        socklen_t length=sizeof(actual);
        // Declared before the native owners, so this runs only after complete
        // context/loop/global teardown. Close only the same remaining socket.
        if(getsockopt(fd,SOL_SOCKET,SO_COOKIE,&actual,&length)==0 && length==sizeof(actual) && actual==cookie) {
            close(fd);
        }
    }
};
void early_fd_error(const char* path) {
    ProbeSocket input{connect_socket(path)};
    input.cookie=socket_cookie(input.fd);
    ProbeInit initialized;
    ProbeGraph graph;
    graph.loop=pw_main_loop_new(nullptr);
    require(graph.loop!=nullptr,"fd probe main loop creation failed");
    auto* properties=pw_properties_new(PW_KEY_PROTOCOL,"mfs-no-such-test-protocol",nullptr);
    require(properties!=nullptr,"fd probe properties allocation failed");
    // Context consumes the properties. No second free on failure.
    graph.context=pw_context_new(pw_main_loop_get_loop(graph.loop),properties,0);
    require(graph.context!=nullptr,"fd probe context creation failed");
    const int fd=input.fd;
    input.offered=true; // ownership is no longer assumed after the native call
    errno=0;
    graph.core=pw_context_connect_fd(graph.context,fd,nullptr,0);
    const int result_errno=errno;
    require(graph.core==nullptr,"fd error injection did not reject the protocol");
    errno=0;
    const int flags=fcntl(fd,F_GETFD);
    const bool open=flags>=0;
    if(open) {
        // Reclaim only the same socket that this isolated probe owned. Never
        // close a descriptor number that native code consumed and reused.
        require(socket_cookie(fd)==input.cookie,"native failure reused the input descriptor");
    } else {
        require(errno==EBADF,"cannot observe input descriptor after native failure");
    }
    require(result_errno!=0,"native failure did not preserve errno");
    // Ownership on early error is observed, not assumed from the type/contract.
    std::cout<<"connect_fd_early_error result=null errno="<<result_errno<<" input_fd_open="<<open<<" observed\n";
}
struct SourceOwner {
    void operator()(Source* source) const noexcept {
        delete source; // original destructor stops/destroys its resources
        // The source constructor called pw_init. This is test-process cleanup,
        // outside that destructor, not evidence of original product deinit.
        pw_deinit();
    }
};
void save(const cv::Mat& image,const std::filesystem::path& path) {
    // Persist even a wrong-color image before assertions. Interpret rows only
    // after the exact readable target layout is established.
    require(image.type()==CV_8UC3 && image.cols==WIDTH && image.rows==HEIGHT,"cannot save unsupported source layout");
    std::ofstream output(path,std::ios::binary);
    require(output.good(),"cannot create source pixel artifact");
    constexpr std::streamsize row=WIDTH*3;
    for(int y=0;y<HEIGHT;++y) { output.write(reinterpret_cast<const char*>(image.ptr(y)),row); }
    require(output.good(),"cannot write source pixel artifact");
}
}
int main(int argc,char** argv) {
    try {
        std::cout.setf(std::ios::unitbuf);
        verify_runtime();
        if(argc==3 && std::string_view(argv[1])=="--fd-error") {
            early_fd_error(argv[2]);
            return 0;
        }
        require(argc==4,"usage: source-context <private socket> <node ID> <artifact directory>");
        const std::string_view id=argv[2];
        uint32_t node=0;
        const auto parsed=std::from_chars(id.data(),id.data()+id.size(),node);
        require(parsed.ec==std::errc{} && parsed.ptr==id.data()+id.size(),"invalid explicit node ID");
        const std::filesystem::path artifacts=argv[3];
        std::filesystem::create_directories(artifacts);
        // Initialize conversion-only runtime outside capture resource accounting.
        cv::Mat raw(HEIGHT,WIDTH,CV_8UC3,cv::Scalar(1,2,3)),converted;
        cv::cvtColor(raw,converted,cv::COLOR_RGB2BGR);
        const size_t base_fds=count_entries("/proc/self/fd");
        const size_t base_threads=count_entries("/proc/self/task");
        std::vector<HeldImage> held;
        held.reserve(60);
        uint32_t previous=0;
        for(int cycle=0;cycle<20;++cycle) {
            std::cout<<"source begin cycle="<<cycle<<'\n';
            const int fd=connect_socket(argv[1]);
            // Transfer the descriptor at the original constructor boundary.
            // Its native connect call consumes it. No second close on error.
            Source* created=nullptr;
            try { created=new Source(fd,node); }
            catch(...) { close(fd);throw; }
            std::unique_ptr<Source,SourceOwner> source(created);
            require(source->init(),"original source init failed");
            require(source->connected(),"original source not connected after init");
            for(int call=0;call<3;++call) {
                std::cout<<"source begin cycle="<<cycle<<" call="<<call<<'\n';
                if(call==1) { source->inactive(); }
                cv::Mat image;
                const auto start=std::chrono::steady_clock::now();
                require(source->screencap(image),"original source screencap failed");
                const auto elapsed=std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::steady_clock::now()-start).count();
                std::cout<<"source observation cycle="<<cycle<<" call="<<call<<" width="<<image.cols
                    <<" height="<<image.rows<<" type="<<image.type()<<" stride="<<image.step[0]<<'\n';
                if(image.type()==CV_8UC3 && image.cols==WIDTH && image.rows==HEIGHT) {
                    save(image,artifacts/("cycle-"+std::to_string(cycle)+"-call-"+std::to_string(call)+".bgr"));
                }
                const uint32_t marker=known_pixels(image);
                const char* relation=previous==0 ? "first" : marker==previous ? "repeat" : marker>previous ? "advance" : "regress";
                held.push_back({std::move(image),marker});
                std::cout<<"source cycle="<<cycle<<" call="<<call<<" elapsed_us="<<elapsed
                    <<" output=known_bgr fixture_marker="<<marker<<" marker_relation="<<relation<<" api_replay=not_exposed\n";
                previous=marker;
            }
            source->close();
            require(!source->connected(),"original source still connected after close");
            source.reset();
            errno=0;
            require(fcntl(fd,F_GETFD)==-1 && errno==EBADF,"source input socket still open after destruction");
            const size_t fds=count_entries("/proc/self/fd"),threads=count_entries("/proc/self/task");
            std::cout<<"source closed cycle="<<cycle<<" fds="<<fds<<" baseline_fds="<<base_fds
                <<" threads="<<threads<<" baseline_threads="<<base_threads<<'\n';
            require(fds==base_fds && threads==base_threads,"source native resource count mismatch");
            for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"retained source sample identity changed"); }
        }
        std::exception_ptr cross_thread_error;
        std::thread reader([&] {
            try { for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"cross-thread source sample identity changed"); } }
            catch(...) { cross_thread_error=std::current_exception(); }
        });
        reader.join();
        if(cross_thread_error) { std::rethrow_exception(cross_thread_error); }
        save(held.front().image,artifacts/"first-after-shutdown.bgr");
        save(held.back().image,artifacts/"last-after-shutdown.bgr");
        std::cout<<"original PipeWire source cycles=20 captures=60 retained=60 passed\n";
        return 0;
    } catch(const std::exception& error) {
        std::cerr<<"source context failure: "<<error.what()<<'\n';
        return 1;
    } catch(...) {
        std::cerr<<"source context unknown exception\n";
        return 2;
    }
}
