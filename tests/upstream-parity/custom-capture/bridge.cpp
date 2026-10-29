// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "bridge.h"
#include "MaaUtils/Buffer/ImageBuffer.hpp"
#include <atomic>
#include <limits>
#include <memory>
#include <stdexcept>
#include <type_traits>
namespace MfsCustomLog {
struct Null { template<class T> const Null& operator<<(const T&) const noexcept {return *this;} };
inline const Null sink;
}
#define LogFunc MfsCustomLog::sink
#define LogTrace MfsCustomLog::sink
#define LogError MfsCustomLog::sink
#define VAR(x) x
#define VAR_VOIDP(x) x
namespace MfsCustomSource {
using MAA_NS::ImageBuffer;
struct CustomControlUnitMgr {
    CustomControlUnitMgr(MaaCustomControllerCallbacks*,void*);
    bool connect();bool connected() const;bool inactive();bool screencap(cv::Mat&);
    MaaCustomControllerCallbacks* controller_=nullptr;
    void* controller_arg_=nullptr;
};
#include "custom-source.inc"
}
#include "custom-raw-source.inc"
namespace {
std::atomic<uint64_t> images=0,contexts=0,entered=0,exited=0;
struct Image {
    cv::Mat value;
    explicit Image(cv::Mat v):value(std::move(v)) {++images;}
    ~Image() {--images;}
};
struct Context {
    bool has_table;
    MaaCustomControllerCallbacks table{};
    MfsCustomSource::CustomControlUnitMgr manager;
    explicit Context(bool valid):has_table(valid),manager(valid?&table:nullptr,nullptr) {++contexts;}
    ~Context() {--contexts;}
};
struct Scope {
    Scope(){++entered;}
    ~Scope(){++exited;}
};
template<class F> int32_t guarded(F&& f) noexcept {
    try {return f();}catch(const cv::Exception&){return 2;}
    catch(const std::exception&){return 3;}catch(...){return 4;}
}
MaaBool native_fault(void*,MaaImageBuffer* buffer) {
    // Real allocated temporary storage must unwind before the exception is
    // contained. This is a fault control, never a successful/retried capture.
    buffer->set(cv::Mat(5,7,CV_8UC3,cv::Scalar(1,2,3)));
    throw std::runtime_error("controlled original custom callback exception");
}
}
size_t mfs_custom_abi(uint32_t index) noexcept {
    static_assert(sizeof(MaaBool)==1 && sizeof(MfsCustomBool)==sizeof(void*) && sizeof(MfsCustomShot)==sizeof(void*));
    const size_t values[]={sizeof(MfsCustomView),alignof(MfsCustomView),offsetof(MfsCustomView,data),
        offsetof(MfsCustomView,length),offsetof(MfsCustomView,stride),offsetof(MfsCustomView,width),
        offsetof(MfsCustomView,height),offsetof(MfsCustomView,type),offsetof(MfsCustomView,channels),
        offsetof(MfsCustomView,references),sizeof(MaaBool),CV_8UC1,CV_8UC3,CV_8UC4,
        sizeof(MaaCustomControllerCallbacks),offsetof(MaaCustomControllerCallbacks,screencap),sizeof(void*)};
    return index<std::size(values)?values[index]:std::numeric_limits<size_t>::max();
}
int32_t mfs_custom_create(uint8_t table,void** output) noexcept {
    if(!output || table>1)return 1;*output=nullptr;
    return guarded([&]{*output=new Context(table!=0);return 0;});
}
void mfs_custom_destroy(void* context) noexcept {delete static_cast<Context*>(context);}
int32_t mfs_custom_configure(void* context,MfsCustomBool connect,MfsCustomBool connected,
    MfsCustomBool inactive,MfsCustomShot shot,void* argument) noexcept {
    if(!context)return 1;auto& c=*static_cast<Context*>(context);
    c.table.connect=connect;c.table.connected=connected;c.table.inactive=inactive;c.table.screencap=shot;
    c.manager.controller_arg_=argument;return 0;
}
int32_t mfs_custom_boolean(void* context,uint32_t op,uint8_t source,uint8_t* value) noexcept {
    if(!context || !value || op>2 || source>1)return 1;*value=0;
    return guarded([&]{auto& c=*static_cast<Context*>(context);
        if(source){*value=op==0?c.manager.connect():op==1?c.manager.connected():c.manager.inactive();return 0;}
        const auto callback=op==0?c.table.connect:op==1?c.table.connected:c.table.inactive;
        if(!c.has_table || !callback)return -1;
        *value=callback(c.manager.controller_arg_);return 0;});
}
uint8_t mfs_custom_has_capture(void* context) noexcept {
    if(!context)return 0;const auto& c=*static_cast<Context*>(context);return c.has_table&&c.table.screencap;
}
int32_t mfs_custom_invoke(void* context,MaaImageBuffer* buffer,uint8_t* result) noexcept {
    if(!context || !buffer || !result)return 1;*result=0;
    return guarded([&]{auto& c=*static_cast<Context*>(context);
        if(!c.has_table || !c.table.screencap)return -1;
        *result=c.table.screencap(c.manager.controller_arg_,buffer);return 0;});
}
int32_t mfs_custom_scope(MfsCustomScope action,void* argument) noexcept {
    if(!action)return 1;
    return guarded([&]{Scope scope;MAA_NS::ImageBuffer buffer;return action(argument,&buffer);});
}
int32_t mfs_custom_raw_set(MaaImageBuffer* buffer,void* data,size_t length,
    int32_t width,int32_t height,int32_t type,uint8_t* result) noexcept {
    if(!result)return 1;*result=0;
    return guarded([&]{
        // The private safe adapter validates its supplied readable span. The
        // original raw setter below remains unchanged and is really invoked.
        if(data && width>0 && height>0){
            // Only these actual header-compiled 8-bit fixture inputs are
            // admitted by this safety adapter, not a general legacy-type API.
            if(type!=CV_8UC1 && type!=CV_8UC3 && type!=CV_8UC4)return 1;
            const size_t element=CV_ELEM_SIZE(type),w=static_cast<size_t>(width),h=static_cast<size_t>(height);
            if(element==0 || w>SIZE_MAX/element || h>SIZE_MAX/(w*element) || w*element*h>length)return 1;
        }
        *result=MaaImageBufferSetRawData(buffer,data,width,height,type);return 0;});
}
int32_t mfs_custom_image(void* buffer,void** output) noexcept {
    if(!output)return 1;*output=nullptr;
    return guarded([&]{cv::Mat value;
        if(buffer)value=static_cast<MaaImageBuffer*>(buffer)->get();
        *output=new Image(std::move(value));return 0;});
}
void mfs_custom_release(void* image) noexcept {delete static_cast<Image*>(image);}
int32_t mfs_custom_seed(void** output) noexcept {
    if(!output)return 1;*output=nullptr;
    return guarded([&]{*output=new Image(cv::Mat(5,7,CV_8UC3,cv::Scalar(9,17,23)));return 0;});
}
int32_t mfs_custom_view(void* image,MfsCustomView* output) noexcept {
    if(!image || !output)return 1;*output={};
    return guarded([&]{const auto& m=static_cast<Image*>(image)->value;
        size_t span=0;
        if(!m.empty()) {
            if(m.cols<=0 || m.rows<=0)return 1;
            const auto width=static_cast<size_t>(m.cols),height=static_cast<size_t>(m.rows),element=m.elemSize();
            if(element==0 || width>SIZE_MAX/element)return 1;
            const auto row=width*element;
            if(m.step[0]<row || (height-1)>(SIZE_MAX-row)/m.step[0])return 1;
            span=(height-1)*m.step[0]+row;
        }
        *output={m.data,span,m.step[0],m.cols,m.rows,m.type(),m.channels(),m.u?m.u->refcount:0};return 0;});
}
int32_t mfs_custom_source(void* context,void* image,uint8_t* result) noexcept {
    if(!context || !image || !result)return 1;*result=0;
    return guarded([&]{*result=static_cast<Context*>(context)->manager.screencap(static_cast<Image*>(image)->value);return 0;});
}
int32_t mfs_custom_fault(void* context) noexcept {
    if(!context)return 1;static_cast<Context*>(context)->table.screencap=native_fault;return 0;
}
uint64_t mfs_custom_count(uint32_t kind) noexcept {
    return kind==0?images.load():kind==1?contexts.load():kind==2?entered.load():kind==3?exited.load():UINT64_MAX;
}
