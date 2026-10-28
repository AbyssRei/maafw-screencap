// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Exact source control bodies, real cv::Mat owners/conversion, explicitly fake
// native loop and clock. No Linux runtime, SPA numeric ABI or capture claim.
#include <algorithm>
#include <chrono>
#include <cstdint>
#include <iostream>
#include <optional>
#include <stdexcept>
#include <utility>
#include <vector>
#include <opencv2/core.hpp>
#include <opencv2/imgproc.hpp>

namespace {
// Symbolic test identifiers, not the numeric SPA ABI.
enum spa_video_format {
    SPA_VIDEO_FORMAT_BGRA, SPA_VIDEO_FORMAT_BGRx, SPA_VIDEO_FORMAT_RGBA,
    SPA_VIDEO_FORMAT_RGBx, SPA_VIDEO_FORMAT_RGB, SPA_VIDEO_FORMAT_BGR,
    SPA_VIDEO_FORMAT_YUY2, SPA_VIDEO_FORMAT_UNKNOWN
};
struct Step { uint64_t elapsed; int result, event, token, format; };
struct Plan { int operation, throws; std::vector<Step> steps; };
struct PipeWireScreencap;
struct Context {
    PipeWireScreencap* state=nullptr;
    uint64_t time=0;
    bool locked=false;
    Plan* plan=nullptr;
    size_t step=0;
    std::vector<std::pair<int,int>> events;
    int conversion_token=0;
} *current=nullptr;
void record(int kind,int value=0) { current->events.emplace_back(kind,value); }
struct pw_thread_loop {} loop;
struct pw_stream {} stream;
void pw_thread_loop_lock(pw_thread_loop*) {
    if(current->locked) { throw std::logic_error("recursive test lock"); }
    current->locked=true; record(1);
}
void pw_thread_loop_unlock(pw_thread_loop*) {
    if(!current->locked) { std::terminate(); }
    record(2); current->locked=false;
}
int pw_stream_set_active(pw_stream*,bool active) {
    if(!current->locked) { throw std::logic_error("active outside lock"); }
    record(3,active); return -5; // source intentionally ignores the return code
}
int pw_thread_loop_timed_wait(pw_thread_loop*,int seconds);
struct FakeClock {
    using time_point=std::chrono::steady_clock::time_point;
    static time_point now() {
        record(4);
        return time_point(std::chrono::nanoseconds(current->time));
    }
};
struct NullLog { template<class T> const NullLog& operator<<(const T&) const { return *this; } } null_log;
struct PipeWireScreencap {
    bool connected_=true,frame_wanted_=false;
    int frame_timeout_count_=0;
    pw_thread_loop* pw_thread_loop_=&loop;
    pw_stream* pw_stream_=&stream;
    cv::Mat latest_frame_;
    spa_video_format frame_format_=SPA_VIDEO_FORMAT_BGRA;
    bool connected() const;
    void inactive();
    bool wait_for_frame_locked(int);
    bool screencap(cv::Mat&);
};
void latest(PipeWireScreencap& state,int token,int format) {
    state.frame_format_=static_cast<spa_video_format>(format);
    if(token==0) { state.latest_frame_={}; return; }
    const int channels=format<4 ? 4 : format==6 ? 2 : 3;
    state.latest_frame_=cv::Mat(1,2,CV_8UC(channels),cv::Scalar(token,80,120,255));
}
int pw_thread_loop_timed_wait(pw_thread_loop*,int seconds) {
    if(!current->locked || current->step>=current->plan->steps.size()) {
        throw std::logic_error("unexpected test wait");
    }
    record(5,seconds);
    const auto step=current->plan->steps[current->step++];
    current->time+=step.elapsed;
    if(step.event==1) {
        latest(*current->state,step.token,step.format);
        current->state->frame_wanted_=false;
    } else if(step.event==2) {
        current->state->connected_=false; // actual source error does not signal
    } else if(step.event==3) {
        current->state->frame_wanted_=false; // predicate success even disconnected
        current->state->connected_=false;
    } else if(step.event==4) {
        current->state->frame_format_=static_cast<spa_video_format>(step.format);
    }
    return step.result;
}
}
namespace ModelStd {
using std::max; using std::move;
namespace chrono {
using std::chrono::seconds; using std::chrono::duration_cast;
using steady_clock=FakeClock;
}
}
namespace ModelCv {
using cv::Mat;
void cvtColor(const cv::Mat& input,cv::Mat& output,int code) {
    if(current->locked) { throw std::logic_error("conversion while loop locked"); }
    const int mode=code==cv::COLOR_BGRA2BGR ? 1 : code==cv::COLOR_RGBA2BGR ? 2
        : code==cv::COLOR_RGB2BGR ? 3 : code==cv::COLOR_YUV2BGR_YUY2 ? 4 : -1;
    if(mode<0) { throw std::logic_error("unexpected conversion mode"); }
    record(6,mode);
    current->conversion_token=input.empty() ? 0 : input.data[0];
    if(current->plan->throws) { throw std::runtime_error("controlled conversion exception"); }
    cv::cvtColor(input,output,code);
}
using cv::COLOR_BGRA2BGR; using cv::COLOR_RGBA2BGR;
using cv::COLOR_RGB2BGR; using cv::COLOR_YUV2BGR_YUY2;
}
namespace {
#define std ModelStd
#define cv ModelCv
#define LogError null_log
#define LogWarn null_log
#define LogDebug null_log
#define VAR(x) x
#include "pipewire-control-source.inc"
#undef VAR
#undef LogDebug
#undef LogWarn
#undef LogError
#undef cv
#undef std
}
int main() {
    try {
        int cases=0;
        if(!(std::cin>>cases) || cases<1 || cases>1000) { return 1; }
        for(int c=0;c<cases;++c) {
            int connected,token,format,timeouts,native,operations;
            if(!(std::cin>>connected>>token>>format>>timeouts>>native>>operations)
                ||format<0 ||format>7 ||operations<1 ||operations>16) { return 1; }
            PipeWireScreencap state;
            state.connected_=connected!=0;
            state.frame_timeout_count_=timeouts;
            if(!native) { state.pw_stream_=nullptr; state.pw_thread_loop_=nullptr; }
            latest(state,token,format);
            Context context; context.state=&state; current=&context;
            for(int op=0;op<operations;++op) {
                Plan plan; int count;
                if(!(std::cin>>plan.operation>>plan.throws>>count)||count<0||count>16) { return 1; }
                for(int i=0;i<count;++i) {
                    Step step;
                    if(!(std::cin>>step.elapsed>>step.result>>step.event>>step.token>>step.format)) { return 1; }
                    plan.steps.push_back(step);
                }
                context.plan=&plan; context.step=0; context.events.clear();context.conversion_token=0;
                cv::Mat output;
                int status=1;
                try {
                    if(plan.operation==1) { state.inactive(); }
                    else { status=state.screencap(output) ? 1 : 0; }
                } catch(...) { status=2; }
                if(context.locked) { throw std::logic_error("source failed to unlock"); }
                std::cout<<status<<' '<<state.frame_timeout_count_<<' '<<state.frame_wanted_<<' '
                    <<state.connected_<<' '<<output.empty()<<' '
                    <<(!output.empty() && output.data==state.latest_frame_.data)<<' '
                    <<context.conversion_token<<' '<<context.step;
                for(const auto& [kind,value]:context.events) { std::cout<<' '<<kind<<':'<<value; }
                std::cout<<'\n';
            }
            current=nullptr;
        }
        return 0;
    } catch(const std::exception& error) {
        std::cerr<<error.what()<<'\n';return 2;
    }
}
