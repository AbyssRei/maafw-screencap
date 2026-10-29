// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#include "Screencap/ScreenCaptureKitScreencap.h"
#include <opencv2/core.hpp>
#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <climits>
#include <cstdlib>
#include <exception>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <libproc.h>
#include <limits>
#include <mach/mach.h>
#include <optional>
#include <pthread.h>
#include <stdexcept>
#include <string>
#include <sys/proc_info.h>
#include <thread>
#include <unistd.h>
#include <vector>

namespace {
constexpr int width=47,height=31;
using Clock=std::chrono::steady_clock;
void require(bool condition,const char* message) {
    if(!condition) { throw std::runtime_error(message); }
}
void pump() {
    @autoreleasepool {
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode
            beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.005]];
    }
}
struct Window {
    NSWindow* value=nullptr;
    explicit Window(NSApplication* app,int cycle) {
        require([NSThread isMainThread],"fixture window created off process main thread");
        value=[[NSWindow alloc] initWithContentRect:NSMakeRect(100+cycle,100,width,height)
            styleMask:NSWindowStyleMaskBorderless backing:NSBackingStoreBuffered defer:NO];
        require(value!=nil,"fixture window allocation failed");
        [value setReleasedWhenClosed:NO];[value setHasShadow:NO];[value setOpaque:YES];
        [value orderFrontRegardless];
        paint(app,0);
    }
    Window(const Window&)=delete;
    Window& operator=(const Window&)=delete;
    ~Window() {
        if(value) { [value close];[value release]; }
    }
    uint32_t id() const {
        const auto number=[value windowNumber];
        require(number>0 && static_cast<uint64_t>(number)<=UINT32_MAX,"fixture window ID narrowing");
        return static_cast<uint32_t>(number);
    }
    void paint(NSApplication* app,int index) {
        const auto colour=index%3;
        [value setBackgroundColor:[NSColor colorWithSRGBRed:(colour==1 ? 1.0 : 0.0)
            green:(colour==0 ? 1.0 : 0.0) blue:(colour==2 ? 1.0 : 0.0) alpha:1.0]];
        [value display];[app updateWindows];
        // Fixture presentation only, before the original capture call begins.
        const auto until=Clock::now()+std::chrono::milliseconds(100);
        while(Clock::now()<until) { pump(); }
    }
};
void pixels(const cv::Mat& image,int colour) {
    require(image.type()==CV_8UC3 && image.cols==width && image.rows==height,
        "original macOS controlled-window layout mismatch");
    const std::array<cv::Vec3b,3> expected={cv::Vec3b(0,255,0),cv::Vec3b(0,0,255),cv::Vec3b(255,0,0)};
    // Match the explicit P0 fixture premise: edge geometry is separately pending.
    for(int y=2;y<height-2;++y) { for(int x=2;x<width-2;++x) {
        require(image.at<cv::Vec3b>(y,x)==expected[static_cast<size_t>(colour%3)],
            "original macOS known interior pixel mismatch");
    } }
}
void save(const cv::Mat& image,const std::filesystem::path& path) {
    require(image.type()==CV_8UC3 && image.dims==2 && image.cols>0 && image.rows>0,
        "cannot save an empty or unexpected original image");
    const auto row=static_cast<size_t>(image.cols)*3;
    require(row<=static_cast<size_t>(std::numeric_limits<std::streamsize>::max()),"image row narrowing");
    std::ofstream output(path,std::ios::binary);
    for(int y=0;y<image.rows;++y) {
        output.write(reinterpret_cast<const char*>(image.ptr(y)),static_cast<std::streamsize>(row));
    }
    output.close();require(output.good(),"cannot save original macOS image");
}
struct Observation {
    size_t descriptors=0,threads=0,ports=0;
};
Observation resources() {
    Observation result;
    const int bytes=proc_pidinfo(getpid(),PROC_PIDLISTFDS,0,nullptr,0);
    require(bytes>=0,"cannot query own descriptor table");
    const auto capacity=static_cast<size_t>(bytes)/sizeof(proc_fdinfo)+32;
    require(capacity<=static_cast<size_t>(INT_MAX)/sizeof(proc_fdinfo),"descriptor table narrowing");
    std::vector<proc_fdinfo> fds(capacity);
    const int used=proc_pidinfo(getpid(),PROC_PIDLISTFDS,0,fds.data(),static_cast<int>(fds.size()*sizeof(proc_fdinfo)));
    require(used>=0 && static_cast<size_t>(used)%sizeof(proc_fdinfo)==0,"cannot read own descriptor table");
    result.descriptors=static_cast<size_t>(used)/sizeof(proc_fdinfo);
    thread_act_array_t threads=nullptr;mach_msg_type_number_t thread_count=0;
    require(task_threads(mach_task_self(),&threads,&thread_count)==KERN_SUCCESS,"cannot enumerate own threads");
    result.threads=thread_count;
    bool released=true;
    for(mach_msg_type_number_t index=0;index<thread_count;++index) {
        if(mach_port_deallocate(mach_task_self(),threads[index])!=KERN_SUCCESS) { released=false; }
    }
    if(vm_deallocate(mach_task_self(),reinterpret_cast<vm_address_t>(threads),
        static_cast<vm_size_t>(thread_count)*sizeof(thread_t))!=KERN_SUCCESS) { released=false; }
    require(released,"cannot release thread observation rights/storage");
    mach_port_name_array_t names=nullptr;mach_port_type_array_t types=nullptr;
    mach_msg_type_number_t name_count=0,type_count=0;
    require(mach_port_names(mach_task_self(),&names,&name_count,&types,&type_count)==KERN_SUCCESS,
        "cannot enumerate own Mach port names");
    result.ports=name_count;
    const auto names_result=vm_deallocate(mach_task_self(),reinterpret_cast<vm_address_t>(names),
        static_cast<vm_size_t>(name_count)*sizeof(mach_port_name_t));
    const auto types_result=vm_deallocate(mach_task_self(),reinterpret_cast<vm_address_t>(types),
        static_cast<vm_size_t>(type_count)*sizeof(mach_port_type_t));
    require(names_result==KERN_SUCCESS && types_result==KERN_SUCCESS,"cannot release port observation storage");
    return result;
}
std::optional<cv::Mat> capture(MfsOriginalMac::ScreenCaptureKitScreencap& source) {
    std::optional<cv::Mat> image;
    std::exception_ptr error;
    std::atomic<bool> done=false;
    uint64_t caller=0;
    const auto start=Clock::now();
    std::thread worker([&] {
        @autoreleasepool {
            pthread_threadid_np(nullptr,&caller);
            try { image=source.screencap(); } catch(...) { error=std::current_exception(); }
        }
        done.store(true,std::memory_order_release);
    });
    while(!done.load(std::memory_order_acquire)) {
        if(Clock::now()-start>std::chrono::seconds(8)) {
            // No detach/cancellation of an active native call or borrowed stack.
            // End this isolated failed fixture instead of destroying its owners.
            std::cerr<<"original macOS caller did not return within fixture watchdog\n"<<std::flush;
            std::_Exit(3);
        }
        pump();
    }
    worker.join();if(error) { std::rethrow_exception(error); }
    std::cout<<"original caller="<<caller<<" elapsed_us="
        <<std::chrono::duration_cast<std::chrono::microseconds>(Clock::now()-start).count()
        <<" engaged="<<image.has_value()<<" empty="<<(image ? image->empty() : true)<<'\n';
    return image;
}
void windows(NSApplication* app,const std::filesystem::path& directory) {
    {
        Window window(app,0);MfsOriginalMac::ScreenCaptureKitScreencap source(window.id());
        auto image=capture(source);require(image.has_value(),"original warmup capture failed");
        save(*image,directory/"warmup.bgr");pixels(*image,0);
    }
    std::ofstream observations(directory/"resources.tsv");
    require(observations.good(),"cannot open native resource observations");
    // Keep this fixture's TSV descriptor present in every observation.
    const auto baseline=resources();
    observations<<"cycle\tfd_count\tthread_count\tport_name_count\n"
        <<"baseline\t"<<baseline.descriptors<<'\t'<<baseline.threads<<'\t'<<baseline.ports<<'\n';
    std::vector<cv::Mat> held;held.reserve(60);
    for(int cycle=0;cycle<20;++cycle) {
        {
            Window window(app,cycle);MfsOriginalMac::ScreenCaptureKitScreencap source(window.id());
            for(int call=0;call<3;++call) {
                window.paint(app,call);
                auto image=capture(source);require(image.has_value(),"original window capture returned nullopt");
                save(*image,directory/("cycle-"+std::to_string(cycle)+"-call-"+std::to_string(call)+".bgr"));
                pixels(*image,call);held.push_back(std::move(*image));
            }
        }
        const auto current=resources();
        observations<<cycle<<'\t'<<current.descriptors<<'\t'<<current.threads<<'\t'<<current.ports<<'\n';
        for(size_t index=0;index<held.size();++index) { pixels(held[index],static_cast<int>(index%3)); }
        std::cout<<"original macOS cycle="<<cycle<<" held="<<held.size()
            <<" fds="<<current.descriptors<<" threads="<<current.threads<<" ports="<<current.ports<<'\n';
    }
    observations.close();require(observations.good(),"cannot save native resource observations");
    // Original not-found return, without manufacturing a null native pointer.
    MfsOriginalMac::ScreenCaptureKitScreencap missing(UINT32_MAX);
    require(!capture(missing).has_value(),"nonexistent original window unexpectedly captured");
    std::exception_ptr error;
    std::thread reader([&] { try {
        for(size_t index=0;index<held.size();++index) { pixels(held[index],static_cast<int>(index%3)); }
    } catch(...) { error=std::current_exception(); } });
    reader.join();if(error) { std::rethrow_exception(error); }
    std::cout<<"original macOS windows cycles=20 captures=60 retained=60 missing_window=1 joined_callers=1 passed\n";
}
void display(const std::filesystem::path& directory) {
    cv::Mat held;
    {
        MfsOriginalMac::ScreenCaptureKitScreencap source(0);
        auto image=capture(source);require(image.has_value() && !image->empty(),"original first-display capture failed");
        save(*image,directory/"first-display.bgr");held=std::move(*image);
    }
    std::ofstream metadata(directory/"first-display.txt");
    metadata<<"width="<<held.cols<<"\nheight="<<held.rows<<"\nchannels=3\n"
        <<"scope=Original first enumerated display; layout/copy observation only, no known-color desktop acceptance.\n";
    metadata.close();require(metadata.good(),"cannot save first-display metadata");
    require(held.type()==CV_8UC3 && held.data!=nullptr,"first-display owner did not survive source destruction");
    std::cout<<"original macOS display captures=1 retained=1 known_pixels=not_asserted passed\n";
}
}
int main(int argc,char** argv) {
    try {
        std::cout.setf(std::ios::unitbuf);
        require(argc==3,"usage: source-context <windows|display> <artifact directory>");
        require([NSThread isMainThread],"AppKit context requires the process main thread");
        if(!CGPreflightScreenCaptureAccess()) {
            std::cerr<<"missing_environment: Screen Recording authorization unavailable; no prompt requested\n";
            return 77;
        }
        @autoreleasepool {
            NSApplication* app=[NSApplication sharedApplication];
            require([app setActivationPolicy:NSApplicationActivationPolicyAccessory],"fixture activation policy rejected");
            [app finishLaunching];
            const std::filesystem::path directory=argv[2];std::filesystem::create_directories(directory);
            std::ofstream build(directory/"opencv-build.txt");
            build<<cv::getBuildInformation()<<"\nCPU features: "<<cv::getCPUFeaturesLine()<<'\n';
            build.close();require(build.good(),"cannot save actual OpenCV optimization information");
            require(cv::useOptimized() && cv::checkHardwareSupport(CV_CPU_NEON),"native NEON/OpenCV optimization unavailable");
            const std::string mode=argv[1];
            if(mode=="windows") { windows(app,directory); }
            else if(mode=="display") { display(directory); }
            else { throw std::runtime_error("unknown original macOS context mode"); }
        }
        return 0;
    }catch(const std::exception& error) { std::cerr<<"original macOS context failure: "<<error.what()<<'\n';return 1; }
    catch(...) { std::cerr<<"original macOS context unknown exception\n";return 2; }
}
