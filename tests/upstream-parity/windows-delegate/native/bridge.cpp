// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "bridge.h"
#include "MaaControlUnit/Win32ControlUnitAPI.h"
#include <windows.h>
#include <array>
#include <vector>
#include <memory>
#include <mutex>
#include <limits>
#include <stdexcept>
#include <cstring>
namespace {
std::mutex mutex;
std::array<MfsDnEvent,4096> events;
size_t event_count=0;
bool overflow=false;
uint64_t owners=0;
uint32_t mode=0;
MfsDnHost host={};
size_t host_live=0;
struct Image { std::vector<uint8_t> bytes; };
int32_t copy(const cv::Mat& mat,MfsDnImage& output) {
    if(mat.empty()) { output={};return 0; }
    if(mat.dims!=2 || mat.type()!=CV_8UC3 || mat.rows<=0 || mat.cols<=0) { return 4; }
    const auto row=static_cast<size_t>(mat.cols)*3;
    if(mat.step[0]<row || static_cast<size_t>(mat.rows)>std::numeric_limits<size_t>::max()/row) { return 4; }
    auto image=std::make_unique<Image>();
    image->bytes.resize(row*static_cast<size_t>(mat.rows));
    for(int y=0;y<mat.rows;++y) { std::memcpy(image->bytes.data()+row*y,mat.ptr(y),row); }
    output={image.get(),image->bytes.data(),image->bytes.size(),mat.cols,mat.rows,row};
    image.release();
    { std::lock_guard lock(mutex);++owners; }
    return 0;
}
}
void mfs_dn_event(uint32_t kind,uint64_t a,uint64_t b,uint64_t c,uint64_t d) noexcept {
    std::lock_guard lock(mutex);
    if(event_count==events.size()) { overflow=true;return; }
    events[event_count++]={kind,GetCurrentThreadId(),a,b,c,d};
}
int32_t mfs_dn_create(uintptr_t address,uintptr_t hwnd,uint64_t method,uint64_t mouse,uint64_t keyboard,void** output) noexcept {
    if(!address || !output) { return 1; } *output=nullptr;
    try { *output=reinterpret_cast<decltype(&MaaWin32ControlUnitCreate)>(address)(reinterpret_cast<void*>(hwnd),method,mouse,keyboard);return 0; }
    catch(const std::bad_alloc&) { return 3; } catch(...) { return 2; }
}
int32_t mfs_dn_destroy(uintptr_t address,void* unit) noexcept {
    if(!address || !unit) { return 1; }
    try { reinterpret_cast<decltype(&MaaWin32ControlUnitDestroy)>(address)(static_cast<MaaWin32ControlUnitHandle>(unit));return 0; }
    catch(const std::bad_alloc&) { return 3; } catch(...) { return 2; }
}
int32_t mfs_dn_connect(void* unit,int32_t* output) noexcept {
    if(!unit || !output) { return 1; } *output=0;
    try { *output=static_cast<MaaWin32ControlUnitHandle>(unit)->connect();return 0; }
    catch(const std::bad_alloc&) { return 3; } catch(...) { return 2; }
}
int32_t mfs_dn_inactive(void* unit,int32_t* output) noexcept {
    if(!unit || !output) { return 1; } *output=0;
    try { *output=static_cast<MaaWin32ControlUnitHandle>(unit)->inactive();return 0; }
    catch(const std::bad_alloc&) { return 3; } catch(...) { return 2; }
}
static int32_t query_impl(void* unit,MfsDnQueryResult* output,bool observe) noexcept {
    if(!unit || !output) { return 1; }
    try {
        auto* api=static_cast<MaaWin32ControlUnitHandle>(unit);
        MfsDnQueryResult result={};
        result.connected=api->connected();
        if(observe) {mfs_dn_event(20,reinterpret_cast<uintptr_t>(unit),result.connected,0,0);}
        std::string uuid="caller sentinel";
        result.uuid_success=api->request_uuid(uuid);
        if(observe) {mfs_dn_event(21,reinterpret_cast<uintptr_t>(unit),result.uuid_success,uuid.size(),0);}
        result.features=api->get_features();
        if(observe) {mfs_dn_event(22,reinterpret_cast<uintptr_t>(unit),result.features,0,0);}
        const auto info=json::value(api->get_info()).to_string();
        if(observe) {mfs_dn_event(23,reinterpret_cast<uintptr_t>(unit),info.size(),0,0);}
        if(uuid.size()>std::size(result.uuid) || info.size()>std::size(result.info)) { return 4; }
        result.uuid_length=uuid.size();result.info_length=info.size();
        std::copy(uuid.begin(),uuid.end(),result.uuid);
        std::copy(info.begin(),info.end(),result.info);
        // These are four separate real virtual calls. Atomic publication here
        // belongs only to this CPU diagnostic snapshot, not to a source SDK.
        *output=result;return 0;
    } catch(const std::bad_alloc&) { return 3; } catch(...) { return 2; }
}
int32_t mfs_dn_query(void* unit,MfsDnQueryResult* output) noexcept {return query_impl(unit,output,false);}
int32_t mfs_dn_query_observed(void* unit,MfsDnQueryResult* output) noexcept {return query_impl(unit,output,true);}
uintptr_t mfs_dn_query_window_create() noexcept {
    // Message-only STATIC window: real HWND lifetime, no visible desktop or
    // capture fixture. The calling thread owns creation and destruction.
    const auto hwnd=CreateWindowExW(0,L"STATIC",L"mfs query lifetime",0,0,0,0,0,HWND_MESSAGE,nullptr,GetModuleHandleW(nullptr),nullptr);
    if(hwnd) { mfs_dn_event(12,reinterpret_cast<uintptr_t>(hwnd),0,0,0); }
    return reinterpret_cast<uintptr_t>(hwnd);
}
int32_t mfs_dn_query_window_destroy(uintptr_t value) noexcept {
    auto hwnd=reinterpret_cast<HWND>(value);
    if(!hwnd || GetWindowThreadProcessId(hwnd,nullptr)!=GetCurrentThreadId()) { return 1; }
    if(!DestroyWindow(hwnd)) { return 2; }
    mfs_dn_event(13,value,0,0,0);return 0;
}
int32_t mfs_dn_query_window_valid(uintptr_t value) noexcept { return IsWindow(reinterpret_cast<HWND>(value))?1:0; }
int32_t mfs_dn_capture(void* unit,MfsDnImage* output,int32_t* success) noexcept {
    if(!unit || !output || !success) { return 1; } *success=0;
    try {
        // Preserve caller output including post-write false/exception paths.
        // Input bytes live in this bridge; local Mat header borrows them only
        // during this call. Any provider Mat is released before returning.
        cv::Mat mat;
        if(output->owner) { mat=cv::Mat(output->height,output->width,CV_8UC3,const_cast<uint8_t*>(output->data),output->stride); }
        const auto* previous=mat.data;
        const auto previous_rows=mat.rows,previous_cols=mat.cols,previous_type=mat.type();
        const auto previous_step=mat.step[0];
        int32_t status=0;
        try { *success=static_cast<MaaWin32ControlUnitHandle>(unit)->screencap(mat); }
        catch(const std::bad_alloc&) { status=3; } catch(...) { status=2; }
        if(mat.data!=previous || mat.empty() || mat.rows!=previous_rows || mat.cols!=previous_cols || mat.type()!=previous_type || mat.step[0]!=previous_step) {
            MfsDnImage next={};
            const auto copied=copy(mat,next);
            if(copied) { return copied; }
            mfs_dn_release(output->owner);*output=next;
        }
        // In-place writes to borrowed bridge bytes already affect the output.
        return status;
    } catch(const std::bad_alloc&) { return 3; } catch(...) { return 2; }
}
void mfs_dn_release(void* image) noexcept {
    if(!image) { return; } delete static_cast<Image*>(image);
    std::lock_guard lock(mutex);--owners;
}
int32_t mfs_dn_events(MfsDnEvent* output,size_t capacity,size_t* count,uint64_t* live) noexcept {
    if(!count || !live || !output) { return 1; }
    std::lock_guard lock(mutex);*count=event_count;*live=owners;
    if(overflow || capacity<event_count) { return 4; }
    std::copy_n(events.begin(),event_count,output);return 0;
}
int32_t mfs_dn_reset() noexcept {
    std::lock_guard lock(mutex);if(owners) { return 4; }event_count=0;overflow=false;mode=0;return 0;
}
uint32_t mfs_dn_mode() noexcept { std::lock_guard lock(mutex);return mode; }
void mfs_dn_set_mode(uint32_t value) noexcept { std::lock_guard lock(mutex);mode=value; }
int32_t mfs_dn_layout(size_t* output,size_t count) noexcept {
    const size_t values[]={sizeof(MfsDnImage),alignof(MfsDnImage),offsetof(MfsDnImage,owner),offsetof(MfsDnImage,data),offsetof(MfsDnImage,length),offsetof(MfsDnImage,width),offsetof(MfsDnImage,height),offsetof(MfsDnImage,stride),sizeof(MfsDnEvent),alignof(MfsDnEvent),sizeof(MfsDnHost),alignof(MfsDnHost),offsetof(MfsDnHost,user),offsetof(MfsDnHost,create),offsetof(MfsDnHost,destroy),offsetof(MfsDnHost,connect),offsetof(MfsDnHost,capture),offsetof(MfsDnHost,inactive),offsetof(MfsDnHost,query),
        sizeof(MfsDnQueryValue),alignof(MfsDnQueryValue),offsetof(MfsDnQueryValue,data),offsetof(MfsDnQueryValue,length),offsetof(MfsDnQueryValue,scalar),
        sizeof(MfsDnQueryResult),alignof(MfsDnQueryResult),offsetof(MfsDnQueryResult,connected),offsetof(MfsDnQueryResult,uuid_success),offsetof(MfsDnQueryResult,features),offsetof(MfsDnQueryResult,uuid_length),offsetof(MfsDnQueryResult,info_length),offsetof(MfsDnQueryResult,uuid),offsetof(MfsDnQueryResult,info)};
    if(!output || count!=std::size(values)) { return 1; }std::copy(std::begin(values),std::end(values),output);return 0;
}
int32_t mfs_dn_host_install(const MfsDnHost* value,size_t size) noexcept {
    std::lock_guard lock(mutex);
    if(host_live) { return 4; }
    if(!value) { host={};return 0; }
    if(size!=sizeof(MfsDnHost) || !value->user || !value->create || !value->destroy || !value->connect || !value->capture || !value->inactive || !value->query) { return 1; }
    host=*value;return 0;
}
int32_t mfs_dn_host_create(uintptr_t hwnd,uint64_t method,void** output) noexcept {
    MfsDnHost callbacks;{ std::lock_guard lock(mutex);callbacks=host; }
    if(!callbacks.create) { return 1; }
    const auto code=callbacks.create(callbacks.user,hwnd,method,output);
    if(!code && *output) { std::lock_guard lock(mutex);++host_live; }
    return code;
}
int32_t mfs_dn_host_destroy(void* context) noexcept {
    MfsDnHost callbacks;{ std::lock_guard lock(mutex);callbacks=host; }
    if(!callbacks.destroy) { return 1; }
    const auto code=callbacks.destroy(callbacks.user,context);
    if(!code) { std::lock_guard lock(mutex);--host_live; }
    return code;
}
int32_t mfs_dn_host_connect(void* context,int32_t* output) noexcept {
    MfsDnHost callbacks;{ std::lock_guard lock(mutex);callbacks=host; }
    return callbacks.connect?callbacks.connect(callbacks.user,context,output):1;
}
int32_t mfs_dn_host_capture(void* context,MfsDnImage* output,int32_t* success) noexcept {
    MfsDnHost callbacks;{ std::lock_guard lock(mutex);callbacks=host; }
    return callbacks.capture?callbacks.capture(callbacks.user,context,output,success):1;
}
int32_t mfs_dn_host_inactive(void* context,int32_t* output) noexcept {
    MfsDnHost callbacks;{ std::lock_guard lock(mutex);callbacks=host; }
    return callbacks.inactive?callbacks.inactive(callbacks.user,context,output):1;
}
int32_t mfs_dn_host_query(void* context,uint32_t selector,MfsDnQueryValue* output) noexcept {
    MfsDnHost callbacks;{ std::lock_guard lock(mutex);callbacks=host; }
    return callbacks.query?callbacks.query(callbacks.user,context,selector,output):1;
}
