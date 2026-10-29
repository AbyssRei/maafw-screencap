// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "Screencap/WlrScreencap.h"
#include <opencv2/imgproc.hpp>
#include <array>
#include <algorithm>
#include <cerrno>
#include <cstring>
#include <dlfcn.h>
#include <exception>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <memory>
#include <stdexcept>
#include <sys/random.h>
#include <thread>
#include <vector>

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
std::string make_uuid() {
    // External reference-only name provider. Every byte comes from getrandom;
    // version/variant and hyphen positions match the usual random UUID shape.
    std::array<unsigned char,16> bytes{};
    size_t offset=0;
    while(offset<bytes.size()) {
        const auto read=getrandom(bytes.data()+offset,bytes.size()-offset,0);
        if(read<0 && errno==EINTR) { continue; }
        require(read>0,"reference UUID getrandom failed");
        offset+=static_cast<size_t>(read);
    }
    bytes[6]=static_cast<unsigned char>((bytes[6]&15)|64);
    bytes[8]=static_cast<unsigned char>((bytes[8]&63)|128);
    constexpr char digits[]="0123456789abcdef";
    std::string result;result.reserve(36);
    for(size_t i=0;i<bytes.size();++i) {
        if(i==4 || i==6 || i==8 || i==10) { result.push_back('-'); }
        result.push_back(digits[bytes[i]>>4]);result.push_back(digits[bytes[i]&15]);
    }
    return result;
}
}

int main(int argc,char** argv) {
    try {
        std::cout.setf(std::ios::unitbuf);
        require(argc==4,"usage: source-context <private socket> <artifact dir> <libwayland path>");
        Dl_info owner{};
        // The linked wrapper has its own address; inspect the real target.
        require(dladdr(reinterpret_cast<void*>(&__real_wl_display_roundtrip),&owner)!=0 && owner.dli_fname,
            "cannot identify actual native roundtrip target");
        require(std::filesystem::canonical(owner.dli_fname)==std::filesystem::canonical(argv[3]),
            "original Wlr loaded a different libwayland-client");
        std::cout<<"native roundtrip owner="<<owner.dli_fname<<'\n';
        const std::filesystem::path directory=argv[2];
        std::filesystem::create_directories(directory);
        // Initialize only conversion runtime before native resource baselines.
        cv::Mat warm(48,64,CV_8UC4,cv::Scalar(0,0,255,255)),converted;
        cv::cvtColor(warm,converted,cv::COLOR_BGRA2BGR);
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
