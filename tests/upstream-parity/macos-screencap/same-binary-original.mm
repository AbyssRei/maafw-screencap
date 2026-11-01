// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Actual unchanged source class in the same executable/process as the Rust
// caller. No source instrumentation, screenshot retry or capture substitution.
#include "same-binary-scene.h"
#include "Screencap/ScreenCaptureKitScreencap.h"
#include <opencv2/core.hpp>
#include <array>
#include <atomic>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <limits>
#include <memory>
#include <optional>
#include <pthread.h>
#include <stdexcept>
#include <string>
#include <unistd.h>

namespace {
std::atomic<uint64_t> owners=0;
uint64_t thread_id() {
    uint64_t value=0;
    if(pthread_threadid_np(nullptr,&value)!=0 || value==0) {throw std::runtime_error("original caller thread identity unavailable");}
    return value;
}
struct Original {
    uint64_t thread=thread_id();
    uint32_t calls=0;
    bool finished=false;
    std::unique_ptr<MfsOriginalMac::ScreenCaptureKitScreencap> source=std::make_unique<MfsOriginalMac::ScreenCaptureKitScreencap>(0);
    std::array<std::optional<cv::Mat>,3> images;
    Original() {++owners;}
    ~Original() {--owners;}
};
void save(const cv::Mat& image,const std::filesystem::path& path) {
    if(image.empty() || image.dims!=2 || image.type()!=CV_8UC3 || image.cols<=0 || image.rows<=0 || !image.data) {
        throw std::runtime_error("original caller output layout differs");
    }
    const auto width=static_cast<size_t>(image.cols);
    if(width>static_cast<size_t>(std::numeric_limits<std::streamsize>::max())/3) {throw std::runtime_error("original row narrowing");}
    const auto row=static_cast<std::streamsize>(width*3);
    std::ofstream out(path,std::ios::binary);out.exceptions(std::ios::badbit|std::ios::failbit);
    for(int y=0;y<image.rows;++y) {out.write(reinterpret_cast<const char*>(image.ptr(y)),row);}
    out.close();
}
}
uint32_t mfs_mac_same_binary_original_version(void) noexcept {return 1;}
uint64_t mfs_mac_same_binary_original_owners(void) noexcept {return owners.load();}
int32_t mfs_mac_same_binary_original_create(void** output) noexcept {
    if(!output) {return 1;}*output=nullptr;
    try {*output=new Original;return 0;}catch(...) {return 2;}
}
int32_t mfs_mac_same_binary_original_capture(void* pointer,uint32_t call,const char* directory) noexcept {
    if(!pointer || !directory) {return 1;}
    try {
        auto& owner=*static_cast<Original*>(pointer);
        if(owner.thread!=thread_id() || owner.finished || call!=owner.calls || call>=3) {return 1;}
        ++owner.calls; // A failing call cannot be silently resubmitted.
        const auto begin=std::chrono::steady_clock::now();
        @autoreleasepool {owner.images[call]=owner.source->screencap();}
        const auto elapsed=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-begin).count();
        const auto& image=owner.images[call];
        const int result=!image ? 0 : image->empty() ? 3 : 1;
        const std::filesystem::path root(directory);
        // Persist the original return before validating/saving its CPU pixels.
        std::ofstream state(root/("original-call-"+std::to_string(call)+".tsv"));state.exceptions(std::ios::badbit|std::ios::failbit);
        state<<call<<'\t'<<result<<'\t'<<elapsed<<'\t'<<owner.thread<<'\t'<<getpid()<<'\n';state.close();
        if(result==1) {
            save(*image,root/("original-call-"+std::to_string(call)+".bgr"));
            std::ofstream layout(root/("original-call-"+std::to_string(call)+"-layout.tsv"));layout.exceptions(std::ios::badbit|std::ios::failbit);
            layout<<image->cols<<'\t'<<image->rows<<'\t'<<image->step[0]<<'\t'<<image->type()<<'\t'<<CV_8UC3<<'\n';layout.close();
        }
        return 0;
    }catch(...) {return 2;}
}
int32_t mfs_mac_same_binary_original_finish(void* pointer,const char* directory) noexcept {
    if(!pointer || !directory) {return 1;}
    try {
        auto& owner=*static_cast<Original*>(pointer);
        if(owner.thread!=thread_id() || owner.finished || owner.calls!=3) {return 1;}
        owner.finished=true;owner.source.reset();
        unsigned held=0;
        const std::filesystem::path root(directory);
        for(unsigned call=0;call<3;++call) {if(owner.images[call] && !owner.images[call]->empty()) {
            save(*owner.images[call],root/("original-held-"+std::to_string(call)+".bgr"));++held;
        }}
        std::ofstream state(root/"original-finish.txt");state.exceptions(std::ios::badbit|std::ios::failbit);
        state<<"schema=1\ncalls=3\ncaller_thread="<<owner.thread<<"\npid="<<getpid()<<"\nsource_object_destroyed=1\nheld_images="<<held
             <<"\ncallback_drain_proved=0\nmodule_retained_until_process_exit=1\n";state.close();
        return 0;
    }catch(...) {return 2;}
}
int32_t mfs_mac_same_binary_original_destroy(void* pointer) noexcept {
    if(!pointer) {return 1;}
    try {
        auto* owner=static_cast<Original*>(pointer);
        if(owner->thread!=thread_id()) {return 1;}
        delete owner;return 0;
    }catch(...) {return 2;}
}
