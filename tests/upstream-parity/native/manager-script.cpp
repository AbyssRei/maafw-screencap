// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "manager-script.h"
#include "Base/UnitBase.h"
#include "MaaUtils/SafeWindows.hpp"
#include <chrono>
#include <memory>
#include <unordered_map>
#include <opencv2/core.hpp>

namespace {
struct ScriptException {};
struct Context {
    const MfsManagerScript& input;
    MfsManagerReport& output;
    uint64_t time = 0;
    uint32_t valid;
    uint32_t method = 0, call = 0;
};
thread_local Context* current = nullptr;
void event(uint32_t kind, uint32_t method=0, uint32_t call=0) noexcept {
    auto& report=current->output;
    if (report.trace_count >= 256) { report.overflow=1; return; }
    report.events[report.trace_count++]={kind,method,call,0,current->time};
}
const MfsManagerStep& step(uint32_t method, uint32_t call) { return current->input.steps[method-1][call-1]; }
class Allocator final : public cv::MatAllocator {
    const cv::MatAllocator* parent=cv::Mat::getDefaultAllocator();
public:
    cv::UMatData* allocate(int dims, const int* sizes, int type, void* data, size_t* stride,
        cv::AccessFlag flags, cv::UMatUsageFlags usage) const override {
        auto* result=parent->allocate(dims,sizes,type,data,stride,flags,usage);
        result->currAllocator=this;
        result->userdata=reinterpret_cast<void*>(static_cast<uintptr_t>(current->method*16+current->call));
        event(3,current->method,current->call);
        return result;
    }
    bool allocate(cv::UMatData* data, cv::AccessFlag flags, cv::UMatUsageFlags usage) const override {
        return parent->allocate(data,flags,usage);
    }
    void deallocate(cv::UMatData* data) const override {
        const auto token=reinterpret_cast<uintptr_t>(data->userdata);
        const auto method=static_cast<uint32_t>(token/16), call=static_cast<uint32_t>(token%16);
        event(4,method,call);
        current->time += step(method,call).image_drop_ns;
        data->currAllocator=parent;
        parent->deallocate(data);
    }
} allocator;
struct FakeClock {
    using time_point=std::chrono::steady_clock::time_point;
    static time_point now() noexcept {
        event(6);
        return time_point(std::chrono::nanoseconds(static_cast<int64_t>(current->time)));
    }
};
struct NullLog {
    template<class T> const NullLog& operator<<(const T&) const noexcept { return *this; }
} null_log;
}
namespace MfsScriptStd {
using std::shared_ptr; using std::make_shared; using std::unordered_map; using std::move;
namespace chrono {
using std::chrono::milliseconds; using std::chrono::duration_cast;
using steady_clock=FakeClock;
}
}
namespace MfsManagerSource {
using MAA_CTRL_UNIT_NS::ScreencapBase;
// The input=None boundary is explicit. No input implementation is imported or
// exercised; source screenshot methods and caller-DPI order remain unchanged.
struct Stub : ScreencapBase {
    uint32_t method, calls=0;
    Stub(HWND, uint32_t id):method(id) {
        event(1,method);
        if (current->input.fail_create==method) { throw ScriptException(); }
    }
    ~Stub() override { event(5,method); }
    std::optional<cv::Mat> screencap() override {
        if (++calls>4) { throw ScriptException(); }
        event(2,method,calls);
        const auto& value=step(method,calls);
        current->time += value.elapsed_ns;
        if (value.outcome==0) { return std::nullopt; }
        if (value.outcome==2) { return cv::Mat(); }
        if (value.outcome==3) { throw ScriptException(); }
        current->method=method; current->call=calls;
        cv::Mat image;
        image.allocator=&allocator;
        image.create(1,1,CV_8UC3);
        image.at<cv::Vec3b>(0,0)=cv::Vec3b(static_cast<uint8_t>(method),static_cast<uint8_t>(calls),0);
        return image;
    }
    void inactive() override { event(9,method); }
};
#define MFS_STUB(name,id) struct name final : Stub { explicit name(HWND hwnd):Stub(hwnd,id) {} };
MFS_STUB(GdiScreencap,1)
MFS_STUB(FramePoolWithPseudoMinimizeScreencap,2)
MFS_STUB(DesktopDupScreencap,3)
MFS_STUB(DesktopDupWindowScreencap,4)
MFS_STUB(PrintWindowWithPseudoMinimizeScreencap,5)
MFS_STUB(ScreenDCScreencap,6)
#undef MFS_STUB
#include "manager-shim.h"
static DPI_AWARENESS_CONTEXT dpi(DPI_AWARENESS_CONTEXT requested) noexcept {
    event(7);
    if (requested!=DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) { current->output.overflow=1; }
    return current->input.dpi_success ? reinterpret_cast<DPI_AWARENESS_CONTEXT>(1) : nullptr;
}
static BOOL valid(HWND) noexcept { event(8); return current->valid!=0; }
static DWORD last_error() noexcept { return 5; }
// Headers above are already parsed. Only the generated fixed function bodies
// see these narrow test boundaries; real STL container/Mat ownership is used.
#define std MfsScriptStd
#define SetThreadDpiAwarenessContext dpi
#define IsWindow valid
#define GetLastError last_error
#define LogInfo null_log
#define LogWarn null_log
#define LogError null_log
#define LogFunc do {} while(false)
#define VAR(x) x
#define VAR_VOIDP(x) x
#include "manager-time.inc"
#include "manager-source.inc"
#undef VAR_VOIDP
#undef VAR
#undef LogFunc
#undef LogError
#undef LogWarn
#undef LogInfo
#undef GetLastError
#undef IsWindow
#undef SetThreadDpiAwarenessContext
#undef std
}
namespace {
uint32_t selected(const MfsManagerSource::Win32ControlUnitMgr& manager) {
    return manager.screencap_ ? static_cast<MfsManagerSource::Stub*>(manager.screencap_.get())->method : 0;
}
uint32_t connect(MfsManagerSource::Win32ControlUnitMgr& manager) {
    try { return manager.connect() ? 1 : 0; } catch(const ScriptException&) { return 2; }
}
uint32_t capture(MfsManagerSource::Win32ControlUnitMgr& manager, cv::Mat& output) {
    try { return manager.screencap(output) ? 1 : 0; } catch(const ScriptException&) { return 2; }
}
uint32_t token(const cv::Mat& image) {
    if (image.empty()) { return 0; }
    const auto pixel=image.at<cv::Vec3b>(0,0);
    return static_cast<uint32_t>(pixel[0])*256+pixel[1];
}
}
int32_t mfs_manager_script(const MfsManagerScript* input, MfsManagerReport* output) {
    if (!output) { return 1; }
    *output={};
    if (!input || current || input->valid>1 || input->dpi_success>1 || input->fail_create>6 || input->reconnect>1 || input->null_window>1 || input->reserved) { return 1; }
    for(const auto& unit : input->steps) for(const auto& value : unit) {
        if(value.outcome>3 || value.reserved || value.elapsed_ns>1000000000000000ULL || value.image_drop_ns>1000000000000000ULL) { return 1; }
    }
    Context context{*input,*output,0,input->valid};
    current=&context;
    struct Reset { ~Reset() { current=nullptr; } } reset;
    try {
        {
            MfsManagerSource::Win32ControlUnitMgr manager(input->null_window ? nullptr : reinterpret_cast<HWND>(1),input->flags,MaaWin32InputMethod_None,MaaWin32InputMethod_None);
            cv::Mat image(1,1,CV_8UC3,cv::Scalar(255,255,0));
            output->connect_status=connect(manager);
            output->selected=selected(manager);
            output->connected_before=manager.connected();
            output->capture_status[0]=capture(manager,image);
            output->output_tokens[0]=token(image);
            context.valid=0;
            output->connected_after=manager.connected();
            output->capture_status[1]=capture(manager,image);
            output->output_tokens[1]=token(image);
            manager.inactive();
            context.valid=input->valid;
            output->reconnect_status=input->reconnect ? connect(manager) : 3;
            output->selected_after_reconnect=selected(manager);
        }
        return output->overflow ? 2 : 0;
    } catch(...) { return 2; }
}
int32_t mfs_manager_script_layout(uint64_t* output,size_t capacity) {
    const uint64_t values[]={sizeof(MfsManagerScript),offsetof(MfsManagerScript,steps),sizeof(MfsManagerStep),offsetof(MfsManagerStep,elapsed_ns),sizeof(MfsManagerReport),offsetof(MfsManagerReport,events),sizeof(MfsManagerEvent),offsetof(MfsManagerEvent,time_ns),sizeof(MfsManagerGdiReport),offsetof(MfsManagerGdiReport,dpi_v2)};
    if(!output || capacity<10) { return 1; }
    for(size_t index=0;index<10;++index) { output[index]=values[index]; }
    return 0;
}
