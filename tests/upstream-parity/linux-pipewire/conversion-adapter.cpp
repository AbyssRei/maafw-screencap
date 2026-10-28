// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "conversion-adapter.h"
#include <opencv2/core.hpp>
#include <opencv2/imgproc.hpp>
#include <atomic>
#include <limits>
#include <memory>
#include <new>
#include <utility>
namespace {
std::atomic<uint64_t> owners{0};
struct Image {
    explicit Image(cv::Mat&& input): pixels(std::move(input)) { owners.fetch_add(1); }
    ~Image() { owners.fetch_sub(1); }
    cv::Mat pixels;
};
bool multiply(size_t a,size_t b,size_t& result) noexcept {
    if(a!=0 && b>std::numeric_limits<size_t>::max()/a) { return false; }
    result=a*b;return true;
}
bool layout(int32_t width,int32_t height,size_t stride,int32_t channels,size_t length) noexcept {
    if(width<=0 || height<=0 || channels<2 || channels>4) { return false; }
    size_t row=0,last=0;
    if(!multiply(static_cast<size_t>(width),static_cast<size_t>(channels),row) || stride<row ||
       !multiply(static_cast<size_t>(height-1),stride,last) || last>std::numeric_limits<size_t>::max()-row) { return false; }
    const size_t span=last+row;
    return span<=length && span<=static_cast<size_t>(std::numeric_limits<ptrdiff_t>::max());
}
}
int32_t mfs_pw_convert(const uint8_t* input,size_t length,int32_t width,int32_t height,size_t stride,int32_t channels,int32_t mode,void** output) noexcept {
    if(!output) { return 1; }
    *output=nullptr;
    if(!input || !layout(width,height,stride,channels,length)) { return 1; }
    int conversion=-1;
    switch(mode) {
    case 1: conversion=cv::COLOR_BGRA2BGR;break;
    case 2: conversion=cv::COLOR_RGBA2BGR;break;
    case 3: conversion=cv::COLOR_RGB2BGR;break;
    case 4: conversion=cv::COLOR_YUV2BGR_YUY2;break;
    default: return 1;
    }
    try {
        // Only the header borrows input. cvtColor reads it synchronously; the
        // independent destination owns its actual OpenCV allocation. No moved
        // conversion or second full-frame CPU copy is hidden in this adapter.
        const cv::Mat raw(height,width,CV_8UC(channels),const_cast<uint8_t*>(input),stride);
        cv::Mat bgr;
        cv::cvtColor(raw,bgr,conversion);
        auto owner=std::make_unique<Image>(std::move(bgr));
        *output=owner.release();
        return 0;
    } catch(const cv::Exception&) { return 2; }
      catch(const std::bad_alloc&) { return 3; }
      catch(...) { return 4; }
}
int32_t mfs_pw_bgr_view(const void* input,MfsPwBgrView* output) noexcept {
    if(!output) { return 1; }
    *output={};
    if(!input) { return 1; }
    const auto& image=static_cast<const Image*>(input)->pixels;
    if(image.empty() || image.type()!=CV_8UC3 || image.dims!=2 || !image.isContinuous()) { return 1; }
    size_t row=0,length=0;
    if(image.cols<=0 || image.rows<=0 || !multiply(static_cast<size_t>(image.cols),3,row) ||
       image.step[0]!=row || !multiply(row,static_cast<size_t>(image.rows),length) ||
       length>static_cast<size_t>(std::numeric_limits<ptrdiff_t>::max())) { return 1; }
    *output={image.data,length,row,image.cols,image.rows};
    return 0;
}
void mfs_pw_bgr_release(void* owner) noexcept { delete static_cast<Image*>(owner); }
uint64_t mfs_pw_bgr_owners() noexcept { return owners.load(); }
size_t mfs_pw_adapter_layout(uint32_t index) noexcept {
    constexpr size_t values[]={1,sizeof(MfsPwBgrView),alignof(MfsPwBgrView),offsetof(MfsPwBgrView,pixels),offsetof(MfsPwBgrView,length),offsetof(MfsPwBgrView,stride),offsetof(MfsPwBgrView,width),offsetof(MfsPwBgrView,height)};
    return index<sizeof(values)/sizeof(values[0]) ? values[index] : std::numeric_limits<size_t>::max();
}
