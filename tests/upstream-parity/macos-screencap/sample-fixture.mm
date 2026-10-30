// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "sample-fixture.h"
#include <CoreMedia/CoreMedia.h>
#include <CoreVideo/CoreVideo.h>
#include <opencv2/core.hpp>
#include <array>
#include <atomic>
#include <cstring>
#include <fstream>
#include <iterator>
#include <limits>
#include <memory>
#include <new>
#include <pthread.h>
#include <utility>

// Definition is compiled separately from the exact fixed upstream .mm.
namespace MfsOriginalMac { cv::Mat image_from_sample_buffer(CMSampleBufferRef); }
namespace {
struct NativeError { int32_t code; };
std::atomic<int32_t> primary{0},cleanup{0};
std::array<std::atomic<uint64_t>,8> counts{};
std::array<std::atomic<uint64_t>,80> release_threads{};
void cleanup_error(int32_t code) noexcept {int32_t expected=0;cleanup.compare_exchange_strong(expected,code);}
uint64_t thread_id() noexcept {
    uint64_t id=0;if(pthread_threadid_np(nullptr,&id)!=0) {cleanup_error(-1);}
    return id;
}
void count(size_t index) noexcept {
    auto value=counts[index].load();
    do {
        if(value==std::numeric_limits<uint64_t>::max()) {cleanup_error(-75);return;}
    }while(!counts[index].compare_exchange_weak(value,value+1));
}
void require(bool value) { if(!value) {throw NativeError{-22};} }
void native(int32_t code) { if(code!=0) {int32_t expected=0;primary.compare_exchange_strong(expected,code);throw NativeError{code};} }
size_t multiply(size_t a,size_t b) {
    require(b==0 || a<=std::numeric_limits<size_t>::max()/b);return a*b;
}
template<class T> struct CF {
    T value=nullptr;
    CF()=default;CF(const CF&)=delete;CF& operator=(const CF&)=delete;
    void reset() noexcept {if(value) {CFRelease(value);value=nullptr;}}
    ~CF() {reset();}
};
struct Fixture {
    CF<CVPixelBufferRef> image;
    CF<CMVideoFormatDescriptionRef> format;
    CF<CMSampleBufferRef> sample;
    MfsMacSampleInfo info{};
    uint32_t case_id=0,cycle=0;
    bool mutated=false,converted=false;
    Fixture() {count(0);info.owner_thread=thread_id();}
    ~Fixture() {
        if(thread_id()!=info.owner_thread) {cleanup_error(-23);}
        sample.reset();format.reset();image.reset();count(1);
    }
    void owner() const {require(thread_id()==info.owner_thread);}
};
struct WriteLock {
    CVPixelBufferRef image;
    explicit WriteLock(CVPixelBufferRef value):image(value) {
        native(CVPixelBufferLockBaseAddress(image,0));count(6);
    }
    ~WriteLock() {
        const auto code=CVPixelBufferUnlockBaseAddress(image,0);
        if(code!=0) {int32_t expected=0;cleanup.compare_exchange_strong(expected,code);}
        count(7);
    }
};
struct SourceImage {
    cv::Mat image;
    uint32_t index;
    SourceImage(cv::Mat&& value,uint32_t position):image(std::move(value)),index(position) {count(2);}
    ~SourceImage() {release_threads[index].store(thread_id());count(3);}
};
int32_t failure() noexcept {
    try {throw;}
    catch(const NativeError& error) {int32_t expected=0;primary.compare_exchange_strong(expected,error.code);return 2;}
    catch(const std::bad_alloc&) {return 3;}
    catch(const cv::Exception&) {return 4;}
    catch(...) {return 5;}
}
std::array<uint8_t,4> pixel(size_t x,size_t y,uint32_t case_id,uint32_t cycle) {
    // All operands are bounded by the four fixed extents and cycle<20. The
    // modulo is the fixture byte pattern, never unchecked layout narrowing.
    constexpr std::array<uint8_t,3> alpha={0,127,255};
    return {static_cast<uint8_t>((x*17+y*3+cycle*11+case_id*5)%256),
        static_cast<uint8_t>((x*7+y*19+cycle*13+case_id*11)%256),
        static_cast<uint8_t>((x*23+y*5+cycle*3+case_id*17)%256),
        alpha[(x+y+cycle+case_id)%3]};
}
}
int32_t mfs_mac_sample_create(uint32_t case_id,uint32_t cycle,void** output) noexcept {
    if(!output) {return 1;}*output=nullptr;
    if(case_id>=4 || cycle>=20) {return 1;}
    try {
        constexpr std::array<size_t,4> widths={7,47,65,1},heights={5,31,3,1};
        constexpr std::array<int64_t,4> alignments={64,64,128,256};
        auto owner=std::make_unique<Fixture>();owner->case_id=case_id;owner->cycle=cycle;
        CF<CFNumberRef> number;
        const auto alignment=alignments[case_id];
        number.value=CFNumberCreate(kCFAllocatorDefault,kCFNumberSInt64Type,&alignment);require(number.value!=nullptr);
        const void* keys[]={kCVPixelBufferBytesPerRowAlignmentKey};
        const void* values[]={number.value};
        CF<CFDictionaryRef> attributes;
        attributes.value=CFDictionaryCreate(kCFAllocatorDefault,keys,values,1,&kCFTypeDictionaryKeyCallBacks,&kCFTypeDictionaryValueCallBacks);
        require(attributes.value!=nullptr);
        native(CVPixelBufferCreate(kCFAllocatorDefault,widths[case_id],heights[case_id],kCVPixelFormatType_32BGRA,attributes.value,&owner->image.value));
        require(owner->image.value!=nullptr && !CVPixelBufferIsPlanar(owner->image.value));
        {
            WriteLock lock(owner->image.value);
            const size_t width=CVPixelBufferGetWidth(owner->image.value),height=CVPixelBufferGetHeight(owner->image.value);
            const size_t stride=CVPixelBufferGetBytesPerRow(owner->image.value),size=CVPixelBufferGetDataSize(owner->image.value);
            require(width==widths[case_id] && height==heights[case_id] && stride%static_cast<size_t>(alignment)==0);
            const size_t row=multiply(width,4),last=multiply(height-1,stride);
            require(stride>row && last<=std::numeric_limits<size_t>::max()-row);
            const auto span=last+row;
            require(span<=size && size<=static_cast<size_t>(std::numeric_limits<ptrdiff_t>::max()));
            owner->info.width=width;owner->info.height=height;owner->info.stride=stride;
            owner->info.span=span;owner->info.data_size=size;owner->info.format=CVPixelBufferGetPixelFormatType(owner->image.value);
            auto* base=static_cast<uint8_t*>(CVPixelBufferGetBaseAddress(owner->image.value));require(base!=nullptr);
            std::memset(base,0xa5,size);
            for(size_t y=0;y<height;++y) {for(size_t x=0;x<width;++x) {
                const auto bytes=pixel(x,y,case_id,cycle);
                // Checked full span and fixed extents prove these offsets.
                std::memcpy(base+y*stride+x*4,bytes.data(),bytes.size());
            }}
        }
        native(CMVideoFormatDescriptionCreateForImageBuffer(kCFAllocatorDefault,owner->image.value,&owner->format.value));
        require(owner->format.value!=nullptr);
        const CMSampleTimingInfo timing={CMTimeMake(1,60),kCMTimeZero,kCMTimeInvalid};
        native(CMSampleBufferCreateReadyWithImageBuffer(kCFAllocatorDefault,owner->image.value,owner->format.value,&timing,&owner->sample.value));
        require(owner->sample.value!=nullptr && CMSampleBufferGetImageBuffer(owner->sample.value)==owner->image.value);
        *output=owner.release();return 0;
    }catch(...) {return failure();}
}
int32_t mfs_mac_sample_info(const void* input,MfsMacSampleInfo* output) noexcept {
    if(!output) {return 1;}*output={};if(!input) {return 1;}
    try {const auto& owner=*static_cast<const Fixture*>(input);owner.owner();*output=owner.info;return 0;}
    catch(...) {return failure();}
}
const void* mfs_mac_sample_buffer(const void* input) noexcept {
    if(!input) {return nullptr;}
    try {
        const auto& owner=*static_cast<const Fixture*>(input);owner.owner();
        require(CFGetTypeID(owner.sample.value)==CMSampleBufferGetTypeID());return owner.sample.value;
    }catch(...) {(void)failure();return nullptr;}
}
int32_t mfs_mac_sample_save(const void* input,const char* path) noexcept {
    if(!input || !path) {return 1;}
    try {
        const auto& owner=*static_cast<const Fixture*>(input);owner.owner();
        // IO retains the real native owner and its read lock; no borrowed pixel
        // view escapes this synchronous fixture function.
        native(CVPixelBufferLockBaseAddress(owner.image.value,kCVPixelBufferLock_ReadOnly));
        struct ReadUnlock {CVPixelBufferRef image;~ReadUnlock() {
            const auto code=CVPixelBufferUnlockBaseAddress(image,kCVPixelBufferLock_ReadOnly);
            if(code!=0) {int32_t expected=0;cleanup.compare_exchange_strong(expected,code);}
        }} unlock{owner.image.value};
        const auto* base=static_cast<const char*>(CVPixelBufferGetBaseAddress(owner.image.value));require(base!=nullptr);
        require(owner.info.data_size<=static_cast<uint64_t>(std::numeric_limits<std::streamsize>::max()));
        std::ofstream file(path,std::ios::binary);file.write(base,static_cast<std::streamsize>(owner.info.data_size));
        file.close();require(file.good());return 0;
    }catch(...) {return failure();}
}
int32_t mfs_mac_sample_mutate(void* input) noexcept {
    if(!input) {return 1;}
    try {
        auto& owner=*static_cast<Fixture*>(input);owner.owner();require(!owner.mutated);
        WriteLock lock(owner.image.value);void* base=CVPixelBufferGetBaseAddress(owner.image.value);require(base!=nullptr);
        std::memset(base,0,static_cast<size_t>(owner.info.data_size));
        owner.mutated=true;owner.info.mutation_thread=thread_id();count(4);return 0;
    }catch(...) {return failure();}
}
void mfs_mac_sample_destroy(void* owner) noexcept {delete static_cast<Fixture*>(owner);}
int32_t mfs_mac_sample_source(void* input,void** output) noexcept {
    if(!output) {return 1;}*output=nullptr;if(!input) {return 1;}
    try {
        auto& owner=*static_cast<Fixture*>(input);owner.owner();require(!owner.mutated && !owner.converted);
        owner.info.source_thread=thread_id();count(5);
        auto image=MfsOriginalMac::image_from_sample_buffer(owner.sample.value);
        require(!image.empty() && image.type()==CV_8UC3 && image.isContinuous());
        auto result=std::make_unique<SourceImage>(std::move(image),owner.cycle*4+owner.case_id);
        owner.converted=true;*output=result.release();return 0;
    }catch(...) {return failure();}
}
int32_t mfs_mac_sample_source_view(const void* input,MfsPwBgrView* output) noexcept {
    if(!output) {return 1;}*output={};if(!input) {return 1;}
    try {
        const auto& image=static_cast<const SourceImage*>(input)->image;
        require(!image.empty() && image.type()==CV_8UC3 && image.isContinuous() && image.cols>0 && image.rows>0);
        const auto row=multiply(static_cast<size_t>(image.cols),3),size=multiply(row,static_cast<size_t>(image.rows));
        require(image.step[0]==row && size<=static_cast<size_t>(std::numeric_limits<ptrdiff_t>::max()));
        *output={image.data,size,row,image.cols,image.rows};return 0;
    }catch(...) {return failure();}
}
void mfs_mac_sample_source_release(void* owner) noexcept {delete static_cast<SourceImage*>(owner);}
size_t mfs_mac_sample_layout(uint32_t index) noexcept {
    constexpr size_t values[]={1,sizeof(MfsMacSampleInfo),alignof(MfsMacSampleInfo),offsetof(MfsMacSampleInfo,width),
        offsetof(MfsMacSampleInfo,height),offsetof(MfsMacSampleInfo,stride),offsetof(MfsMacSampleInfo,span),
        offsetof(MfsMacSampleInfo,data_size),offsetof(MfsMacSampleInfo,format),offsetof(MfsMacSampleInfo,owner_thread),
        offsetof(MfsMacSampleInfo,source_thread),offsetof(MfsMacSampleInfo,mutation_thread)};
    return index<std::size(values) ? values[index] : std::numeric_limits<size_t>::max();
}
uint64_t mfs_mac_sample_counter(uint32_t index) noexcept {return index<counts.size() ? counts[index].load() : UINT64_MAX;}
int32_t mfs_mac_sample_error(uint32_t index) noexcept {return index==0 ? primary.load() : index==1 ? cleanup.load() : -22;}
uint64_t mfs_mac_sample_release_thread(uint32_t index) noexcept {return index<release_threads.size() ? release_threads[index].load() : 0;}
uint64_t mfs_mac_sample_thread() noexcept {return thread_id();}
