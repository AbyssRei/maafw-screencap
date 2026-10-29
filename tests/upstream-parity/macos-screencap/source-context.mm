// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#include "Screencap/ScreenCaptureKitScreencap.h"
#include <opencv2/core.hpp>
#include "resource-observation.h"
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
#include "manager-native.h"

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
using Observation=MfsMacResources;
Observation resources() { return mfs_mac_observe_resources(); }
template<class Source> std::optional<cv::Mat> capture(Source& source) {
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
struct ManagerCaller {
    MfsMacManagerNative::MacOSControlUnitMgr manager;
    explicit ManagerCaller(uint32_t id) : manager(id,MaaMacOSScreencapMethod_ScreenCaptureKit,MaaMacOSInputMethod_None) {}
    std::optional<cv::Mat> screencap() {
        if(!manager.connected()) {
            const auto before=MfsMacManagerNative::calls.load();
            require(manager.connect() && manager.connected(),"original manager connect failed");
            require(MfsMacManagerNative::calls.load()==before,"original connect unexpectedly captured");
        }
        cv::Mat output;
        if(!manager.screencap(output)) { return std::nullopt; }
        require(manager.inactive(),"original manager inactive failed");
        return output;
    }
};
void manager_external_window(const std::filesystem::path& directory,const char* argument) {
    const auto parsed=std::stoull(argument);
    require(parsed>0 && parsed<=UINT32_MAX,"manager fixture window ID narrowing");
    std::vector<cv::Mat> held;held.reserve(60);
    for(int cycle=0;cycle<20;++cycle) {
        {
            ManagerCaller caller(static_cast<uint32_t>(parsed));
            std::string uuid;
            require(caller.manager.request_uuid(uuid) && uuid==std::to_string(parsed),"original manager UUID mismatch");
            for(int call=0;call<3;++call) {
                auto image=capture(caller);require(image.has_value(),"original manager capture failed");
                pixels(*image,0);
                save(*image,directory/("manager-cycle-"+std::to_string(cycle)+"-call-"+std::to_string(call)+".bgr"));
                if(cycle==0 && call==0) { save(*image,directory/"external-window.bgr"); }
                held.push_back(std::move(*image));
            }
        }
        require(MfsMacManagerNative::creates.load()==static_cast<uint64_t>(cycle+1) &&
            MfsMacManagerNative::drops.load()==static_cast<uint64_t>(cycle+1),"original manager source-owner mismatch");
        for(const auto& image:held) { pixels(image,0); }
    }
    require(MfsMacManagerNative::calls.load()==60 && MfsMacManagerNative::input_attempts.load()==0,
        "original manager forwarding or Input=None mismatch");
    std::exception_ptr error;
    std::thread reader([&] { try { for(const auto& image:held) { pixels(image,0); } }
        catch(...) { error=std::current_exception(); } });
    reader.join();if(error) { std::rethrow_exception(error); }
    std::cout<<"original macOS manager cycles=20 captures=60 retained=60 connect_capture_calls=0 source_owners_released=20 input_attempts=0 passed\n";
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
void scene(NSApplication* app,const std::filesystem::path& directory) {
    {
    Window window(app,0);
    const auto temporary=directory/"ready.tmp";
    const auto ready=directory/"ready.json";
    std::ofstream output(temporary);
    output<<"{\"window_id\":"<<window.id()<<",\"width\":47,\"height\":31,\"colour\":\"green\",\"owner_pid\":"<<getpid()<<"}\n";
    output.close();require(output.good(),"cannot publish fixture scene identity");
    std::filesystem::rename(temporary,ready);
    const auto until=Clock::now()+std::chrono::minutes(3);
    while(!std::filesystem::exists(directory/"stop")) {
        if(Clock::now()>=until) { throw std::runtime_error("scene fixture exceeded its isolated lifetime budget"); }
        pump();
    }
    }
    // Publish only after the owning main thread closed/released the window.
    // The Rust reader may then check CPU owners without a mere stop-request race.
    std::ofstream stopped(directory/"stopped.tmp");
    stopped<<"window_closed=1\n";stopped.close();
    require(stopped.good(),"cannot acknowledge scene window close");
    std::filesystem::rename(directory/"stopped.tmp",directory/"stopped.txt");
    std::cout<<"owned macOS scene stop=1 main_thread=1 passed\n";
}
void external_window(const std::filesystem::path& directory,const char* argument) {
    const auto parsed=std::stoull(argument);
    require(parsed>0 && parsed<=UINT32_MAX,"external fixture window ID narrowing");
    MfsOriginalMac::ScreenCaptureKitScreencap source(static_cast<uint32_t>(parsed));
    auto image=capture(source);require(image.has_value(),"original external-window capture failed");
    save(*image,directory/"external-window.bgr");pixels(*image,0);
    std::cout<<"original macOS external_window captures=1 known_interior=1 passed\n";
}
}
int main(int argc,char** argv) {
    try {
        std::cout.setf(std::ios::unitbuf);
        require(argc==3 || argc==4,"usage: source-context <windows|display|scene|external-window> <artifact directory> [window ID]");
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
            else if(mode=="scene") { scene(app,directory); }
            else if(mode=="external-window" && argc==4) { external_window(directory,argv[3]); }
            else if(mode=="manager-external-window" && argc==4) { manager_external_window(directory,argv[3]); }
            else { throw std::runtime_error("unknown original macOS context mode"); }
        }
        return 0;
    }catch(const std::exception& error) { std::cerr<<"original macOS context failure: "<<error.what()<<'\n';return 1; }
    catch(...) { std::cerr<<"original macOS context unknown exception\n";return 2; }
}
