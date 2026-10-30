// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Compile the complete unedited original translation unit here, so its static
// negotiation helpers are available to the explicit POD oracle below.
#include "Screencap/PipeWireScreencap.cpp"
#include "Screencap/NoneScreencap.h"
#include "MaaFramework/MaaDef.h"
#include "fd-fault.h"
#include "dma-fixture.h"
#include <pipewire/pipewire.h>
#include <opencv2/imgproc.hpp>
#include <algorithm>
#include <array>
#include <cerrno>
#include <charconv>
#include <chrono>
#include <cstddef>
#include <cstring>
#include <cstdlib>
#include <exception>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iomanip>
#include <latch>
#include <limits>
#include <memory>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string_view>
#include <thread>
#include <utility>
#include <vector>
#include <fcntl.h>
#include <dlfcn.h>
#include <pthread.h>
#include <sys/socket.h>
#include <sys/mman.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

#if defined(MFS_PW_KWIN_NATIVE_API)
#include "kwin-native-owner.hpp"
#else
namespace MfsOriginalPipeWire {
#define MFS_MANAGER_HOST_INPUT
#include "../linux-manager/native-shim.h"
#undef MFS_MANAGER_HOST_INPUT
}
#endif

namespace {
using Source=MfsOriginalPipeWire::PipeWireScreencap;
constexpr int WIDTH=28,HEIGHT=24;
void require(bool condition,const char* message) {
    if(!condition) { throw std::runtime_error(message); }
}
template<typename Function> Function fault_symbol(const char* name) {
    void* pointer=dlsym(RTLD_DEFAULT,name);
    const char* expected=std::getenv("MFS_P1_PIPEWIRE_FD_FAULT_LIBRARY");
    Dl_info info{};
    require(pointer && expected && dladdr(pointer,&info)!=0 && info.dli_fname,
        "explicit loaded FD fault control is absent");
    require(std::filesystem::canonical(info.dli_fname)==std::filesystem::canonical(expected),
        "FD fault control loaded from a different library");
    return reinterpret_cast<Function>(pointer);
}
struct FdFaultControl {
    decltype(&mfs_pw_fd_fault_arm) arm=fault_symbol<decltype(arm)>("mfs_pw_fd_fault_arm");
    decltype(&mfs_pw_fd_fault_attempts) attempts=fault_symbol<decltype(attempts)>("mfs_pw_fd_fault_attempts");
    decltype(&mfs_pw_fd_fault_rejections) rejections=fault_symbol<decltype(rejections)>("mfs_pw_fd_fault_rejections");
    decltype(&mfs_pw_fd_fault_native_error) native_error=fault_symbol<decltype(native_error)>("mfs_pw_fd_fault_native_error");
    decltype(&mfs_pw_fd_fault_control_error) control_error=fault_symbol<decltype(control_error)>("mfs_pw_fd_fault_control_error");
    void check(uint32_t expected_attempts) const {
        require(attempts()==expected_attempts && rejections()==1 && native_error()==ENOMEM && control_error()==0,
            "native FD fault control attempts/rejection/error mismatch");
    }
};
void verify_runtime() {
    const char* version=std::getenv("MFS_P0_PIPEWIRE_VERSION");
    const char* library=std::getenv("MFS_P1_PIPEWIRE_LIBRARY");
    require(version && library,"explicit native version/library identities are required");
    std::cout<<"source headers="<<pw_get_headers_version()<<" runtime="<<pw_get_library_version()<<" expected="<<version<<'\n';
    require(std::string_view(pw_get_headers_version())==version && std::string_view(pw_get_library_version())==version,"source headers/runtime differ from the selected PipeWire version");
    Dl_info info{};
    void* symbol=dlsym(RTLD_DEFAULT,"pw_init");
    require(symbol && dladdr(symbol,&info)!=0 && info.dli_fname,"cannot identify native PipeWire symbol owner");
    std::cout<<"source symbol_owner="<<info.dli_fname<<" expected_library="<<library<<'\n';
    require(std::filesystem::canonical(info.dli_fname)==std::filesystem::canonical(library),"source loaded a different PipeWire library");
    std::cout<<"source runtime="<<version<<" library="<<info.dli_fname<<'\n';
}
uint32_t known_pixels(const cv::Mat& image) {
    require(image.type()==CV_8UC3 && image.cols==WIDTH && image.rows==HEIGHT,"source output layout mismatch");
    const auto header=image.at<cv::Vec3b>(0,0);
    const uint32_t marker=(static_cast<uint32_t>(header[2])<<16)|(static_cast<uint32_t>(header[1])<<8)|header[0];
    require(marker!=0,"source fixture marker is absent");
    constexpr std::array<std::array<uint8_t,3>,7> rgb={{{191,191,191},{191,191,0},{0,191,191},{0,191,0},{191,0,191},{191,0,0},{0,0,191}}};
    for(int y=0;y<HEIGHT;++y) {
        for(int x=0;x<WIDTH;++x) {
            if(x==0 && y==0) { continue; } // producer serial in RGB byte order
            const auto& base=rgb[static_cast<size_t>(x)/4];
            std::array<uint8_t,3> expected{};
            for(size_t channel=0;channel<3;++channel) {
                expected[channel]=static_cast<uint8_t>(base[channel] ^ ((static_cast<size_t>(y)*7+static_cast<size_t>(x)*3+channel*13+(marker&63))&63));
            }
            const auto pixel=image.at<cv::Vec3b>(y,x);
            require(pixel[0]==expected[2] && pixel[1]==expected[1] && pixel[2]==expected[0],"source known-color mismatch");
        }
    }
    return marker;
}
struct HeldImage { cv::Mat image;uint32_t marker; };
size_t count_entries(const char* path) {
    size_t count=0;
    for([[maybe_unused]] const auto& entry:std::filesystem::directory_iterator(path)) { ++count; }
    return count;
}
std::vector<std::string> task_ids() {
    std::vector<std::string> tasks;
    for(const auto& entry:std::filesystem::directory_iterator("/proc/self/task")) {
        tasks.push_back(entry.path().filename().string());
    }
    std::sort(tasks.begin(),tasks.end());
    return tasks;
}
struct Task {
    std::string id,name;
    uint64_t started,flags,address_space;
    int64_t resident_pages;
};
struct TaskStatReadFailure:std::runtime_error {
    std::string id;
    int native_error;
    bool bad,failed,eof;
    size_t bytes;
    TaskStatReadFailure(const std::string& task_id,int code,bool is_bad,bool is_failed,bool is_eof,size_t length)
        :std::runtime_error("cannot read proc task stat"),id(task_id),native_error(code),
         bad(is_bad),failed(is_failed),eof(is_eof),bytes(length) {}
};
struct TaskAbsence {
    bool stat_opened=false,directory_exists=false;
    int stat_error=0,close_error=0,directory_error=0;
};
bool known_live_source_task(const Task& before) {
    return !before.id.empty() && std::all_of(before.id.begin(),before.id.end(),[](char byte){return byte>='0' && byte<='9';}) &&
        before.name=="MaaScreencap" && before.started!=0 && !(before.flags&0x4) && before.address_space!=0;
}
bool exact_retired_read_failure(const Task& before,const TaskStatReadFailure& failure) {
    return known_live_source_task(before) && failure.id==before.id && failure.native_error==ESRCH &&
        failure.bad && failure.failed && !failure.eof && failure.bytes==0;
}
TaskAbsence probe_task_absence(const std::string& id) {
    // Single fresh native observations, not a retry of the failed stream read.
    // ENOENT on BOTH stat open and task-directory stat is required. All other
    // errors, an opened file or a still-present directory refuse classification.
    const auto directory=std::filesystem::path("/proc/self/task")/id;
    TaskAbsence observed;
    errno=0;
    const int fd=open((directory/"stat").c_str(),O_RDONLY|O_CLOEXEC);
    if(fd>=0) {
        observed.stat_opened=true;
        if(close(fd)!=0) {observed.close_error=errno;}
        return observed;
    }
    observed.stat_error=errno;
    if(observed.stat_error!=ENOENT) {return observed;}
    struct stat info{};
    errno=0;
    if(::stat(directory.c_str(),&info)==0) {observed.directory_exists=true;}
    else {observed.directory_error=errno;}
    return observed;
}
bool proved_task_absent(const TaskAbsence& observed) {
    return !observed.stat_opened && observed.stat_error==ENOENT && observed.close_error==0 &&
        !observed.directory_exists && observed.directory_error==ENOENT;
}
bool same_generation_kernel_retirement(const Task& before,const Task& after) {
    return known_live_source_task(before) && after.id==before.id && after.name==before.name &&
        after.started==before.started && (after.flags&0x4) && after.address_space==0 && after.resident_pages==0;
}
bool observe_post_join_failed_task(const Task& before,const TaskStatReadFailure& failure,TaskAbsence& observed) {
    if(!exact_retired_read_failure(before,failure)) {return false;}
    observed=probe_task_absence(before.id);
    const bool absent=proved_task_absent(observed);
    std::cout<<"post_join_task_read id="<<before.id<<" started="<<before.started
        <<" read_errno="<<failure.native_error<<" bad="<<failure.bad<<" fail="<<failure.failed
        <<" eof="<<failure.eof<<" bytes="<<failure.bytes<<" fresh_stat_open="<<observed.stat_opened
        <<" fresh_stat_errno="<<observed.stat_error<<" fresh_close_errno="<<observed.close_error
        <<" fresh_directory_exists="<<observed.directory_exists
        <<" fresh_directory_errno="<<observed.directory_error<<" proved_absent="<<absent<<'\n';
    return absent;
}
std::string task_text(std::ifstream& input,const std::string& id) {
    std::string text;
    errno=0;
    std::getline(input,text);
    const int read_error=errno;
    if(!input.good() && !input.eof()) {
        // Default/live observations remain strict. Only the separate post-join
        // path may classify this exact evidence with fresh absence proof.
        std::cerr<<"proc task stat read id="<<id<<" errno="<<read_error
            <<" bad="<<input.bad()<<" fail="<<input.fail()<<" eof="<<input.eof()
            <<" bytes="<<text.size()<<'\n';
        throw TaskStatReadFailure(id,read_error,input.bad(),input.fail(),input.eof(),text.size());
    }
    require(input.good() || input.eof(),"cannot read proc task stat");
    return text;
}
std::optional<Task> task(const std::string& id) {
    errno=0;
    std::ifstream input(std::filesystem::path("/proc/self/task")/id/"stat");
    if(!input) { require(errno==ENOENT,"cannot inspect proc task");return std::nullopt; }
    const auto text=task_text(input,id);
    const auto end=text.rfind(')');
    require(end!=std::string::npos,"invalid proc task comm field");
    std::istringstream body(text.substr(end+1));
    std::vector<std::string> fields;
    std::string field;
    while(body>>field) { fields.push_back(field); }
    require(fields.size()>=22,"short proc task stat");
    auto number=[&](size_t index){
        uint64_t result=0;
        const auto& word=fields[index];
        const auto parsed=std::from_chars(word.data(),word.data()+word.size(),result);
        require(parsed.ec==std::errc{} && parsed.ptr==word.data()+word.size(),"invalid proc task number");
        return result;
    };
    // Fields 9/22/23/24, after the final ')' of comm (which can contain spaces).
    int64_t resident=0;
    const auto& word=fields[21];
    const auto parsed=std::from_chars(word.data(),word.data()+word.size(),resident);
    require(parsed.ec==std::errc{} && parsed.ptr==word.data()+word.size(),"invalid proc task RSS");
    const auto begin=text.find('(');
    require(begin!=std::string::npos && begin<end,"invalid proc task name");
    return Task{id,text.substr(begin+1,end-begin-1),number(19),number(6),number(20),resident};
}
std::vector<Task> live_tasks() {
    std::vector<Task> result;
    for(const auto& id:task_ids()) { if(auto value=task(id)) { result.push_back(std::move(*value)); } }
    return result;
}
bool only_retiring_tasks(const std::vector<std::string>& actual,const std::vector<std::string>& baseline,const std::vector<Task>& live) {
    // PRECONDITION: source close/stop/join has returned and native owners have
    // completed their scoped teardown. Every caller is after that boundary.
    // This never waits for active callbacks or permits asynchronous cleanup.
    for(const auto& id:baseline) {
        if(std::find(actual.begin(),actual.end(),id)==actual.end()) { return false; }
    }
    for(const auto& id:actual) {
        if(std::find(baseline.begin(),baseline.end(),id)!=baseline.end()) { continue; }
        const auto before=std::find_if(live.begin(),live.end(),[&](const Task& value){return value.id==id;});
        if(before==live.end() || !known_live_source_task(*before)) { return false; }
        try {
            if(auto after=task(id)) {
                // Only a known same-generation PF_EXITING task with no userspace
                // address space can finish kernel retirement here. Never a live
                // worker, an unknown TID, or asynchronous callback/native cleanup.
                if(!same_generation_kernel_retirement(*before,*after)) { return false; }
            }
        }catch(const TaskStatReadFailure& failure) {
            TaskAbsence observed;
            if(!observe_post_join_failed_task(*before,failure,observed)) {return false;}
        }
    }
    return true;
}
struct TaskRead {
    uint64_t bytes=0;
    int open_error=0,read_error=0,close_error=0;
};
TaskRead write_task_file(std::ostream& output,const std::filesystem::path& path) {
    TaskRead result;
    struct Input {
        int fd;
        ~Input() { if(fd>=0) { close(fd); } }
    } input{open(path.c_str(),O_RDONLY|O_CLOEXEC)};
    const int fd=input.fd;
    if(fd<0) { result.open_error=errno; }
    output<<path<<" open="<<(fd>=0)<<" open_errno="<<result.open_error<<'\n';
    if(fd>=0) {
        std::array<char,4096> bytes{};
        for(;;) {
            const auto count=read(fd,bytes.data(),bytes.size());
            if(count==0) { break; }
            if(count<0) {
                if(errno==EINTR) { continue; }
                result.read_error=errno;break;
            }
            const auto length=static_cast<uint64_t>(count);
            if(result.bytes>UINT64_MAX-length) { result.read_error=EOVERFLOW;break; }
            result.bytes+=length;
            output.write(bytes.data(),static_cast<std::streamsize>(count));
            if(!output) { break; } // retain the real output error, still close fd
        }
        if(close(std::exchange(input.fd,-1))<0) { result.close_error=errno; } // no uncertain close retry
    }
    output<<"\nread_bytes="<<result.bytes<<" read_errno="<<result.read_error
        <<" close_errno="<<result.close_error<<'\n';
    // A disappearing/empty proc input is diagnostic evidence, not an output
    // write failure or proof that a native task/cleanup gate passed.
    return result;
}
void task_snapshot(const std::filesystem::path& directory,const std::string& label,const std::vector<std::string>& tasks) {
    std::ofstream output(directory/("tasks-"+label+".txt"));
    require(output.good(),"cannot write task diagnostic");
    output<<"tasks observed="<<tasks.size()<<'\n';
    for(const auto& tid:tasks) {
        for(const char* name:{"comm","stat","status","wchan"}) {
            const auto path=std::filesystem::path("/proc/self/task")/tid/name;
            const auto read=write_task_file(output,path);
            require(read.close_error==0,"cannot close task diagnostic input");
        }
    }
    output.close();
    require(output.good(),"cannot close task diagnostic");
}
void task_diagnostic_fixtures(const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    std::istringstream empty_input;
    std::ostringstream old_output;
    old_output<<empty_input.rdbuf();
    require(old_output.fail(),"empty streambuf did not reproduce the old output failure");
    const auto empty=directory/"empty.input";
    const auto nonempty=directory/"bytes.input";
    { std::ofstream file(empty);file.close();require(file.good(),"cannot create empty input fixture"); }
    { std::ofstream file(nonempty);file<<"known task bytes\n";file.close();require(file.good(),"cannot create task input fixture"); }
    require(!std::filesystem::exists(directory/"missing.input"),"missing-input fixture already exists");
    std::ostringstream output;
    const auto first=write_task_file(output,empty);
    const auto second=write_task_file(output,nonempty);
    const auto third=write_task_file(output,directory/"missing.input");
    require(output.good() && first.bytes==0 && first.open_error==0 && first.read_error==0 && first.close_error==0,
        "empty diagnostic input contaminated the output state");
    require(second.bytes==17 && second.open_error==0 && second.read_error==0 && second.close_error==0 &&
        output.str().find("known task bytes\n")!=std::string::npos,"nonempty diagnostic input changed");
    require(third.bytes==0 && third.open_error==ENOENT && third.read_error==0 && third.close_error==0,
        "missing diagnostic input was not explicitly recorded");
    std::ofstream failed_output("/dev/full");
    require(failed_output.good(),"cannot open native full-device output control");
    const auto last=write_task_file(failed_output,nonempty);
    errno=0;
    failed_output.flush();
    const int write_error=errno;
    require(failed_output.fail() && write_error==ENOSPC && last.open_error==0 && last.read_error==0 && last.close_error==0,
        "real diagnostic output write failure was lost");
    failed_output.close();
    std::ofstream saved(directory/"observations.txt");
    saved<<output.str();saved.close();require(saved.good(),"cannot save task diagnostic fixtures");
    std::cout<<"native task diagnostic empty_stream_failure=1 empty_file=1 bytes_file=1 missing_file=1 real_output_failure=1 passed\n";
}
void observe_joined_tasks(const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    task_snapshot(directory,"join-control-baseline",task_ids());
    size_t residual=0;
    for(size_t cycle=0;cycle<10000;++cycle) {
        pid_t tid=-1;
        int name_error=-1;
        std::thread worker([&] {
            name_error=pthread_setname_np(pthread_self(),"mfs-join-probe");
            tid=gettid();
        });
        worker.join(); // std::thread throws if the real pthread_join fails.
        require(tid>0 && name_error==0,"joined thread identity unavailable");
        const auto tasks=task_ids();
        if(std::find(tasks.begin(),tasks.end(),std::to_string(tid))==tasks.end()) { continue; }
        ++residual;
        if(residual<=20) {
            std::cout<<"joined task still enumerated cycle="<<cycle<<" tid="<<tid<<'\n';
            task_snapshot(directory,"joined-"+std::to_string(cycle),tasks);
        }
        if(residual==1) {
            for(int delay:{1,10,100}) {
                std::this_thread::sleep_for(std::chrono::milliseconds(delay));
                task_snapshot(directory,"joined-first-after-"+std::to_string(delay)+"ms",task_ids());
            }
        }
    }
    task_snapshot(directory,"join-control-final",task_ids());
    // Observational control only: zero and nonzero residuals are both retained.
    // It cannot excuse a failed capture test or serve as cleanup acceptance.
    std::cout<<"native joined-task observation cycles=10000 residual_enumerations="<<residual<<" observed\n";
}
void task_read_exit_fixture(const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    const auto baseline_fds=count_entries("/proc/self/fd");
    const auto baseline_tasks=task_ids();
    int native_error=0;
    bool strict_rejected=false;
    pid_t tid=-1;
    {
        struct Worker {
            std::latch ready{1},release{1};
            pid_t tid=-1;
            bool released=false;
            std::thread thread;
            Worker():thread([this]{tid=gettid();ready.count_down();release.wait();}) {}
            void finish() {
                if(!released) { released=true;release.count_down(); }
                if(thread.joinable()) { thread.join(); }
            }
            ~Worker() { finish(); }
        } worker;
        worker.ready.wait();
        tid=worker.tid;
        require(tid>0,"cannot obtain exit fixture task identity");
        const auto id=std::to_string(tid);
        const auto path=std::filesystem::path("/proc/self/task")/id/"stat";
        // Both handles open while this gated task is still alive. Neither has
        // read/buffered stat yet; no new task is created before the reads below.
        std::ifstream input(path);
        require(input.good(),"cannot open live exit fixture stat stream");
        struct Descriptor {
            int value;
            ~Descriptor() { if(value>=0) { close(value); } }
        } descriptor{open(path.c_str(),O_RDONLY|O_CLOEXEC)};
        require(descriptor.value>=0,"cannot open live exit fixture stat fd");
        const auto before=task(id);
        require(before.has_value(),"gated exit fixture task disappeared early");
        worker.finish();
        // This deliberately selects the fully retired state for a causal
        // procfs control. It is not a wait/retry in the capture/resource gate.
        const auto deadline=std::chrono::steady_clock::now()+std::chrono::seconds(2);
        while(std::filesystem::exists(path) && std::chrono::steady_clock::now()<deadline) {
            std::this_thread::yield();
        }
        require(!std::filesystem::exists(path),"exit fixture task did not retire");
        std::array<char,4096> bytes{};
        errno=0;
        const auto length=read(descriptor.value,bytes.data(),bytes.size());
        native_error=errno;
        require(length==-1 && native_error==ESRCH,"retired task stat fd did not report ESRCH");
        try { static_cast<void>(task_text(input,id)); }
        catch(const std::runtime_error& error) {
            require(std::string_view(error.what())=="cannot read proc task stat","unexpected strict observer exception");
            strict_rejected=true;
        }
        require(strict_rejected && input.bad() && input.fail() && !input.eof(),
            "strict task observer did not retain its actual read failure");
        require(!task(id).has_value(),"retired fixture task was not absent on a fresh open");
        const int fd=std::exchange(descriptor.value,-1);
        require(close(fd)==0,"cannot close exit fixture native stat fd");
        input.clear();input.close();require(!input.fail(),"cannot close exit fixture stat stream");
    }
    require(count_entries("/proc/self/fd")==baseline_fds && task_ids()==baseline_tasks,
        "task exit fixture resources did not return to baseline");
    std::ofstream saved(directory/"observations.txt");
    saved<<"tid="<<tid<<"\nopened_while_live=1\njoined_before_read=1\npath_absent=1\nread_bytes=-1\nread_errno="<<native_error
        <<"\nstrict_observer_rejected="<<strict_rejected<<"\nfresh_open_absent=1\nresources_balanced=1\n";
    saved.close();require(saved.good(),"cannot save task exit fixture observations");
    std::cout<<"native task read-exit opened_live=1 joined_before_read=1 path_absent=1 read=ESRCH strict_observer_rejected=1 fresh_open_absent=1 resources_balanced=1 passed\n";
}
#include "task-retirement-context.hpp"
int connect_socket(const char* path) {
    sockaddr_un address{};
    address.sun_family=AF_UNIX;
    const size_t length=std::strlen(path);
    require(length>0 && length<sizeof(address.sun_path),"private socket path does not fit sockaddr_un");
    std::memcpy(address.sun_path,path,length+1);
    const int fd=socket(AF_UNIX,SOCK_STREAM|SOCK_CLOEXEC,0);
    require(fd>=0,"source socket creation failed");
    if(connect(fd,reinterpret_cast<const sockaddr*>(&address),sizeof(address))!=0) {
        const int saved=errno;
        close(fd);
        throw std::runtime_error(std::string("private source socket connect failed: ")+std::strerror(saved));
    }
    return fd;
}
uint64_t socket_cookie(int fd) {
    uint64_t cookie=0;
    socklen_t length=sizeof(cookie);
    require(getsockopt(fd,SOL_SOCKET,SO_COOKIE,&cookie,&length)==0 && length==sizeof(cookie),"cannot establish owned socket identity");
    return cookie;
}
struct ProbeInit {
    ProbeInit() { pw_init(nullptr,nullptr); }
    ~ProbeInit() { pw_deinit(); }
};
struct ProbeGraph {
    pw_main_loop* loop=nullptr;
    pw_context* context=nullptr;
    pw_core* core=nullptr;
    ~ProbeGraph() {
        if(core) { pw_core_disconnect(core); }
        if(context) { pw_context_destroy(context); }
        if(loop) { pw_main_loop_destroy(loop); }
    }
};
struct ProbeSocket {
    int fd;
    uint64_t cookie=0;
    bool offered=false;
    ~ProbeSocket() {
        if(fd<0) { return; }
        if(!offered) { close(fd);return; }
        uint64_t actual=0;
        socklen_t length=sizeof(actual);
        // Declared before the native owners, so this runs only after complete
        // context/loop/global teardown. Close only the same remaining socket.
        if(getsockopt(fd,SOL_SOCKET,SO_COOKIE,&actual,&length)==0 && length==sizeof(actual) && actual==cookie) {
            close(fd);
        }
    }
};
void early_fd_error(const char* path) {
    ProbeSocket input{connect_socket(path)};
    input.cookie=socket_cookie(input.fd);
    ProbeInit initialized;
    ProbeGraph graph;
    graph.loop=pw_main_loop_new(nullptr);
    require(graph.loop!=nullptr,"fd probe main loop creation failed");
    auto* properties=pw_properties_new(PW_KEY_PROTOCOL,"mfs-no-such-test-protocol",nullptr);
    require(properties!=nullptr,"fd probe properties allocation failed");
    // Context consumes the properties. No second free on failure.
    graph.context=pw_context_new(pw_main_loop_get_loop(graph.loop),properties,0);
    require(graph.context!=nullptr,"fd probe context creation failed");
    const int fd=input.fd;
    input.offered=true; // ownership is no longer assumed after the native call
    errno=0;
    graph.core=pw_context_connect_fd(graph.context,fd,nullptr,0);
    const int result_errno=errno;
    require(graph.core==nullptr,"fd error injection did not reject the protocol");
    errno=0;
    const int flags=fcntl(fd,F_GETFD);
    const bool open=flags>=0;
    if(open) {
        // Reclaim only the same socket that this isolated probe owned. Never
        // close a descriptor number that native code consumed and reused.
        require(socket_cookie(fd)==input.cookie,"native failure reused the input descriptor");
    } else {
        require(errno==EBADF,"cannot observe input descriptor after native failure");
    }
    require(result_errno!=0,"native failure did not preserve errno");
    // Ownership on early error is observed, not assumed from the type/contract.
    std::cout<<"connect_fd_early_error result=null errno="<<result_errno<<" input_fd_open="<<open<<" observed\n";
}
class AddIoFault {
public:
    AddIoFault(pw_loop* loop,int fd): interface_(&loop->utils->iface), original_(interface_->cb.funcs), expected_fd_(fd) {
        require(active_==nullptr && original_!=nullptr,"invalid isolated add_io fault premise");
        methods_=*static_cast<const spa_loop_utils_methods*>(original_);
        require(methods_.add_io!=nullptr,"native add_io method is absent");
        original_add_io_=methods_.add_io;
        methods_.add_io=reject;
        active_=this;
        // Explicit test-only interception on an owned, non-running main loop.
        // Keep native callback data and all other methods unchanged. Restore
        // before context/loop teardown; no product method table is modified.
        interface_->cb.funcs=&methods_;
    }
    ~AddIoFault() { interface_->cb.funcs=original_;active_=nullptr; }
    AddIoFault(const AddIoFault&)=delete;
    AddIoFault& operator=(const AddIoFault&)=delete;
    uint32_t calls() const { return calls_; }
private:
    static spa_source* reject(void* data,int fd,uint32_t mask,bool close,spa_source_io_func_t callback,void* user) noexcept {
        auto* fault=active_;
        if(!fault) { errno=EINVAL;return nullptr; }
        if(fd!=fault->expected_fd_) { return fault->original_add_io_(data,fd,mask,close,callback,user); }
        if(fault->calls_==UINT32_MAX) { errno=EOVERFLOW;return nullptr; }
        ++fault->calls_;
        errno=ENOMEM;
        return nullptr;
    }
    inline static AddIoFault* active_=nullptr;
    spa_interface* interface_;
    const void* original_;
    spa_loop_utils_methods methods_{};
    decltype(spa_loop_utils_methods::add_io) original_add_io_=nullptr;
    int expected_fd_;
    uint32_t calls_=0;
};
void fd_add_io_error(const char* path) {
    ProbeSocket input{connect_socket(path)};
    input.cookie=socket_cookie(input.fd);
    int result_errno=0;
    bool open=false;
    uint32_t calls=0;
    {
        ProbeInit initialized;
        ProbeGraph graph;
        graph.loop=pw_main_loop_new(nullptr);
        require(graph.loop!=nullptr,"add_io probe loop creation failed");
        graph.context=pw_context_new(pw_main_loop_get_loop(graph.loop),nullptr,0);
        require(graph.context!=nullptr,"add_io probe context creation failed");
        AddIoFault fault(pw_main_loop_get_loop(graph.loop),input.fd);
        input.offered=true;
        errno=0;
        graph.core=pw_context_connect_fd(graph.context,input.fd,nullptr,0);
        result_errno=errno;
        calls=fault.calls();
        require(graph.core==nullptr && result_errno==ENOMEM && calls==1,"late fd error did not reach the intercepted native add_io boundary");
        errno=0;
        open=fcntl(input.fd,F_GETFD)>=0;
        if(open) { require(socket_cookie(input.fd)==input.cookie,"late native failure reused the input descriptor"); }
        else { require(errno==EBADF,"cannot observe late-error input descriptor"); }
    } // restore vtable, then native graph/loop/global cleanup, in that order
    errno=0;
    const bool after=fcntl(input.fd,F_GETFD)>=0;
    if(after) { require(socket_cookie(input.fd)==input.cookie,"native teardown reused the failed input descriptor"); }
    else { require(errno==EBADF,"cannot observe descriptor after native teardown"); }
    std::cout<<"connect_fd_add_io_error result=null errno="<<result_errno<<" input_fd_open="<<open
        <<" after_teardown_fd_open="<<after<<" intercepted_calls="<<calls<<" observed\n";
}
struct SourceOwner {
    void operator()(Source* source) const noexcept {
        delete source; // original destructor stops/destroys its resources
        // The source constructor called pw_init. This is test-process cleanup,
        // outside that destructor, not evidence of original product deinit.
        pw_deinit();
    }
};
void save(const cv::Mat& image,const std::filesystem::path& path) {
    // Persist even a wrong-color image before assertions. Interpret rows only
    // after the exact readable target layout is established.
    require(image.type()==CV_8UC3 && image.cols==WIDTH && image.rows==HEIGHT,"cannot save unsupported source layout");
    std::ofstream output(path,std::ios::binary);
    require(output.good(),"cannot create source pixel artifact");
    constexpr std::streamsize row=WIDTH*3;
    for(int y=0;y<HEIGHT;++y) { output.write(reinterpret_cast<const char*>(image.ptr(y)),row); }
    require(output.good(),"cannot write source pixel artifact");
}
void save_pods(const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    alignas(8) uint8_t bytes[4096]={};
    spa_pod_builder builder=SPA_POD_BUILDER_INIT(bytes,sizeof(bytes));
    const spa_pod* dmabuf=MfsOriginalPipeWire::make_video_format_pod(builder,true);
    const spa_pod* shm=MfsOriginalPipeWire::make_video_format_pod(builder,false);
    // Same buffer-dataType POD expression as original pw_connect_stream.
    int32_t data_types=(1<<SPA_DATA_MemFd)|(1<<SPA_DATA_DmaBuf);
    const spa_pod* buffers=static_cast<const spa_pod*>(spa_pod_builder_add_object(
        &builder,SPA_TYPE_OBJECT_ParamBuffers,SPA_PARAM_Buffers,
        SPA_PARAM_BUFFERS_dataType,SPA_POD_CHOICE_FLAGS_Int(data_types),0));
    const std::array<const spa_pod*,3> pods={dmabuf,shm,buffers};
    for(size_t index=0;index<pods.size();++index) {
        require(pods[index]!=nullptr,"source POD builder failed");
        const size_t size=SPA_POD_SIZE(pods[index]);
        require(size<=sizeof(bytes),"source POD exceeds builder storage");
        std::ofstream output(directory/("param-"+std::to_string(index)+".pod"),std::ios::binary);
        output.write(reinterpret_cast<const char*>(pods[index]),static_cast<std::streamsize>(size));
        require(output.good(),"cannot save original POD");
    }
    std::cout<<"original PipeWire negotiation pods=3 saved\n";
}
void save_bytes(const std::filesystem::path& path,const void* bytes,size_t length) {
    require(length<=static_cast<size_t>(std::numeric_limits<std::streamsize>::max()),"fixture file length overflow");
    std::ofstream output(path,std::ios::binary);
    output.write(static_cast<const char*>(bytes),static_cast<std::streamsize>(length));
    output.close();
    require(output.good(),"cannot save format fixture");
}
void save_format_fixtures(const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    std::ofstream records(directory/"cases.tsv");
    require(records.good(),"cannot create format fixture records");
    records<<"index\tspa_format\tchannels\twidth\theight\tstride\tchoice_none\tlinear_modifier\tnative_parse_result\n";
    size_t index=0;
    for(const auto& info:MfsOriginalPipeWire::kSpaFormatTable) {
        for(bool choice:{false,true}) {
            for(bool modifier:{false,true}) {
                alignas(8) uint8_t bytes[4096]={};
                spa_pod_builder builder=SPA_POD_BUILDER_INIT(bytes,sizeof(bytes));
                spa_pod_frame object{},child{};
                auto checked=[](int result){require(result>=0,"format fixture builder failed");};
                checked(spa_pod_builder_push_object(&builder,&object,SPA_TYPE_OBJECT_Format,SPA_PARAM_Format));
                checked(spa_pod_builder_add(&builder,
                    SPA_FORMAT_mediaType,SPA_POD_Id(SPA_MEDIA_TYPE_video),
                    SPA_FORMAT_mediaSubtype,SPA_POD_Id(SPA_MEDIA_SUBTYPE_raw),0));
                checked(spa_pod_builder_prop(&builder,SPA_FORMAT_VIDEO_format,0));
                if(choice) { checked(spa_pod_builder_push_choice(&builder,&child,SPA_CHOICE_None,0)); }
                checked(spa_pod_builder_id(&builder,info.format));
                if(choice) { require(spa_pod_builder_pop(&builder,&child)!=nullptr,"format fixture choice failed"); }
                checked(spa_pod_builder_prop(&builder,SPA_FORMAT_VIDEO_size,0));
                if(choice) { checked(spa_pod_builder_push_choice(&builder,&child,SPA_CHOICE_None,0)); }
                constexpr uint32_t width=8,height=3;
                checked(spa_pod_builder_rectangle(&builder,width,height));
                if(choice) { require(spa_pod_builder_pop(&builder,&child)!=nullptr,"size fixture choice failed"); }
                if(modifier) {
                    checked(spa_pod_builder_prop(&builder,SPA_FORMAT_VIDEO_modifier,0));
                    checked(spa_pod_builder_long(&builder,DRM_FORMAT_MOD_LINEAR));
                }
                const auto* param=static_cast<const spa_pod*>(spa_pod_builder_pop(&builder,&object));
                require(param && SPA_POD_SIZE(param)<=sizeof(bytes),"format fixture POD is absent/oversize");
                spa_video_info_raw native{};
                const int native_parse=spa_format_video_raw_parse(param,&native);
                uint32_t actual_width=0,actual_height=0;
                spa_video_format actual_format=SPA_VIDEO_FORMAT_UNKNOWN;
                MfsOriginalPipeWire::parse_video_format(param,actual_width,actual_height,actual_format);
                require(actual_width==width && actual_height==height && actual_format==info.format,"original format parser rejected the valid fixture");
                const size_t row=static_cast<size_t>(width)*static_cast<size_t>(info.channels),stride=row+11;
                std::vector<uint8_t> raw(stride*height,0xed);
                for(size_t y=0;y<height;++y) {
                    for(size_t x=0;x<width;++x) {
                        for(int c=0;c<info.channels;++c) {
                            raw[y*stride+x*static_cast<size_t>(info.channels)+static_cast<size_t>(c)]=
                                static_cast<uint8_t>((y*29+x*17+static_cast<size_t>(c)*53+index*19)&255);
                        }
                    }
                }
                const cv::Mat borrowed(height,width,CV_8UC(info.channels),raw.data(),stride);
                cv::Mat owned=borrowed.clone(),bgr;
                // Use the fixed source's actual format table and actual OpenCV
                // operations. This isolated format oracle has no native buffer,
                // callback, capture-time or end-to-end format acceptance claim.
                if(info.cv_conversion>=0) { cv::cvtColor(owned,bgr,info.cv_conversion); }
                else { bgr=std::move(owned); }
                require(bgr.type()==CV_8UC3 && bgr.isContinuous() && bgr.step[0]==width*3,"format oracle output layout mismatch");
                const auto name="case-"+std::to_string(index);
                save_bytes(directory/(name+".pod"),param,SPA_POD_SIZE(param));
                save_bytes(directory/(name+".raw"),raw.data(),raw.size());
                save_bytes(directory/(name+".bgr"),bgr.data,static_cast<size_t>(width)*height*3);
                records<<index<<'\t'<<static_cast<uint32_t>(actual_format)<<'\t'<<info.channels<<'\t'
                    <<actual_width<<'\t'<<actual_height<<'\t'<<stride<<'\t'<<choice<<'\t'<<modifier<<'\t'<<native_parse<<'\n';
                ++index;
            }
        }
    }
    records.close();
    require(index==28 && records.good(),"format fixture records incomplete");
    std::cout<<"original PipeWire format fixtures=28 formats=7 choice_none=14 linear_modifier=14 saved\n";
}
struct MemFile {
    int fd;
    explicit MemFile(int value):fd(value) { require(fd>=0,"memfd fixture creation failed"); }
    MemFile(const MemFile&)=delete;
    MemFile& operator=(const MemFile&)=delete;
    ~MemFile() {
        if(fd>=0 && ::close(fd)!=0) { std::cerr<<"memfd fixture cleanup errno="<<errno<<'\n'; }
    }
    void close() { const int value=fd;fd=-1;require(::close(value)==0,"memfd fixture close failed"); }
};
size_t file_mappings(const std::string& name,bool check_permissions) {
    std::ifstream maps("/proc/self/maps");
    require(maps.good(),"cannot read map observations");
    size_t count=0;
    std::string line;
    while(std::getline(maps,line)) {
        if(line.find("/memfd:"+name+" ")==std::string::npos) { continue; }
        std::istringstream fields(line);
        std::string range,permissions;
        fields>>range>>permissions;
        if(check_permissions) { require(permissions=="r--p","source map is not read-only/private"); }
        ++count;
    }
    return count;
}
void save_memfd_fixtures(const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    std::ofstream layout(directory/"spa-data-layout.txt");
    layout<<sizeof(spa_data)<<' '<<alignof(spa_data)<<' '<<offsetof(spa_data,type)<<' '
        <<offsetof(spa_data,flags)<<' '<<offsetof(spa_data,fd)<<' '<<offsetof(spa_data,mapoffset)<<' '
        <<offsetof(spa_data,maxsize)<<' '<<offsetof(spa_data,data)<<' '<<offsetof(spa_data,chunk)<<' '
        <<sizeof(spa_data::fd)<<'\n';
    layout.close();require(layout.good(),"cannot write native data ABI observations");
    std::ofstream records(directory/"cases.tsv");
    records<<"index\tstride\tmaxsize\tchunk_size\thas_chunk\tfile_length\tmarker\n";
    constexpr size_t row=WIDTH*3;
    constexpr std::array<std::array<uint8_t,3>,7> bars={{{191,191,191},{191,191,0},{0,191,191},{0,191,0},{191,0,191},{191,0,0},{0,0,191}}};
    cv::Mat warm(HEIGHT,WIDTH,CV_8UC3,cv::Scalar(1,2,3)),warm_bgr;
    cv::cvtColor(warm,warm_bgr,cv::COLOR_RGB2BGR);
    const size_t baseline=count_entries("/proc/self/fd");
    std::vector<HeldImage> held;
    held.reserve(60);
    size_t index=0;
    for(size_t cycle=0;cycle<20;++cycle) {
        const auto name="mfs-source-map-"+std::to_string(cycle);
        MemFile file(memfd_create(name.c_str(),MFD_CLOEXEC));
        require(file_mappings(name,false)==0,"source memfd unexpectedly mapped before helper");
        for(size_t mode=0;mode<3;++mode) {
            const size_t stride=mode==2 ? row : 96;
            const size_t capacity=stride*HEIGHT;
            const size_t span=(HEIGHT-1)*stride+row;
            // The chunk fallback maps the declared length but reads no trailing
            // final-row padding: the actual file deliberately ends at span.
            const size_t file_length=mode==1 ? span : capacity;
            const uint32_t marker=static_cast<uint32_t>(index+1);
            std::vector<uint8_t> bytes(capacity,0xed);
            for(size_t y=0;y<HEIGHT;++y) {
                for(size_t x=0;x<WIDTH;++x) {
                    for(size_t c=0;c<3;++c) {
                        bytes[y*stride+x*3+c]=static_cast<uint8_t>(bars[x/4][c]^((y*7+x*3+c*13+(marker&63))&63));
                    }
                }
            }
            bytes[0]=static_cast<uint8_t>(marker>>16);bytes[1]=static_cast<uint8_t>(marker>>8);bytes[2]=static_cast<uint8_t>(marker);
            require(ftruncate(file.fd,static_cast<off_t>(file_length))==0 &&
                    pwrite(file.fd,bytes.data(),file_length,0)==static_cast<ssize_t>(file_length),"cannot initialize memfd fixture");
            spa_chunk chunk{};
            chunk.stride=static_cast<int32_t>(stride);chunk.size=static_cast<uint32_t>(capacity);
            spa_data data{};
            data.type=SPA_DATA_MemFd;data.flags=SPA_DATA_FLAG_READABLE;data.fd=file.fd;
            data.maxsize=mode==1 ? 0 : static_cast<uint32_t>(capacity);
            data.chunk=mode==2 ? nullptr : &chunk;
            cv::Mat raw(HEIGHT,WIDTH,CV_8UC3);
            {
                MfsOriginalPipeWire::ScopedBufferMap map(data,capacity);
                require(map.ptr()!=nullptr,"original MemFd helper failed valid input");
                require(file_mappings(name,true)==1,"original helper mapping not observed");
                if(stride==row) { std::memcpy(raw.data,map.ptr(),HEIGHT*stride); }
                else { for(size_t y=0;y<HEIGHT;++y) { std::memcpy(raw.ptr(static_cast<int>(y)),static_cast<const uint8_t*>(map.ptr())+y*stride,row); } }
            }
            require(file_mappings(name,false)==0,"original helper mapping survived destruction");
            require(fcntl(file.fd,F_GETFD)>=0,"original mapping helper consumed the native FD");
            cv::Mat bgr;
            cv::cvtColor(raw,bgr,cv::COLOR_RGB2BGR); // after owned unmap, as in U
            require(known_pixels(bgr)==marker,"source memfd target pixels differ");
            const auto base="case-"+std::to_string(index);
            save_bytes(directory/(base+".input"),bytes.data(),bytes.size());
            save_bytes(directory/(base+".rgb"),raw.data,HEIGHT*row);
            save_bytes(directory/(base+".bgr"),bgr.data,HEIGHT*row);
            records<<index<<'\t'<<stride<<'\t'<<data.maxsize<<'\t'<<capacity<<'\t'<<(data.chunk!=nullptr)<<'\t'<<file_length<<'\t'<<marker<<'\n';
            held.push_back({std::move(bgr),marker});
            std::fill(bytes.begin(),bytes.end(),0);
            require(pwrite(file.fd,bytes.data(),file_length,0)==static_cast<ssize_t>(file_length),"cannot overwrite released memfd input");
            for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"held memfd BGR changed after native overwrite"); }
            ++index;
        }
        file.close();
        require(count_entries("/proc/self/fd")==baseline,"source memfd fixture descriptor leak");
    }
    records.close();require(records.good() && index==60,"memfd fixture records incomplete");
    for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"held memfd image changed after all FDs closed"); }
    std::cout<<"original MemFd helper cycles=20 maps=60 unmaps=60 retained_bgr=60 readonly_private=60 fd_preserved=60 passed\n";
}
#include "dma-fixtures.hpp"
#include "manager-context.hpp"
#include "manager-host-context.hpp"
#if !defined(MFS_PW_KWIN_NATIVE_API)
#include "manager-host-offer-context.hpp"
#endif
#include "sustained-context.hpp"
void reinitialization_context(const char* socket,uint32_t node,const std::filesystem::path& artifacts,bool inject_failure=false) {
    std::filesystem::create_directories(artifacts);
    std::optional<FdFaultControl> fault;
    if(inject_failure) { fault.emplace(); }
    cv::Mat raw(HEIGHT,WIDTH,CV_8UC3,cv::Scalar(1,2,3)),converted;
    cv::cvtColor(raw,converted,cv::COLOR_RGB2BGR);
    const size_t base_fds=count_entries("/proc/self/fd");
    const auto baseline=task_ids();
    task_snapshot(artifacts,"baseline",baseline);
    std::vector<HeldImage> held;
    held.reserve(120);
    for(int cycle=0;cycle<20;++cycle) {
        const int fd=connect_socket(socket);
        const uint64_t cookie=socket_cookie(fd);
        Source* created=nullptr;
        try { created=new Source(fd,node); }
        catch(...) { close(fd);throw; }
        std::unique_ptr<Source,SourceOwner> source(created);
        // Close before first init retains the offered socket in the source.
        source->close();source->close();source->inactive();
        require(!source->connected() && socket_cookie(fd)==cookie,"pre-init close changed the offered socket");
        cv::Mat empty;
        require(!source->screencap(empty) && empty.empty(),"uninitialized source produced a frame");
        if(fault) {
            require(fault->arm(fd)==0,"cannot arm source FD fault");
            require(!source->init(),"original source accepted the injected native ENOMEM");
            source->close();
            require(!source->connected() && socket_cookie(fd)==cookie,
                "source failed teardown consumed/replaced the offered socket");
            fault->check(1);
            require(count_entries("/proc/self/fd")==base_fds+1 && task_ids()==baseline,
                "failed source init did not clean native resources while retaining its offered socket");
            std::cout<<"source fd-retry cycle="<<cycle<<" failed_errno="<<fault->native_error()
                <<" retained_cookie="<<cookie<<" native_resources_closed=1\n";
        }
        for(int session=0;session<2;++session) {
            require(source->init() && source->connected(),"source same-object init failed");
            if(fault) { fault->check(2); }
            require(source->init(),"source already-connected init failed");
            for(int call=0;call<3;++call) {
                if(call==1) { source->inactive();source->inactive(); }
                cv::Mat image;
                require(source->screencap(image),"source same-object screencap failed");
                save(image,artifacts/("cycle-"+std::to_string(cycle)+"-session-"+std::to_string(session)+"-call-"+std::to_string(call)+".bgr"));
                const uint32_t marker=known_pixels(image);
                held.push_back({std::move(image),marker});
            }
            const auto live=live_tasks();
            source->close();source->close();source->inactive();
            require(!source->connected(),"closed same-object source remained connected");
            cv::Mat rejected=held.back().image;
            const auto* pixels=rejected.data;
            require(!source->screencap(rejected) && rejected.data==pixels &&
                known_pixels(rejected)==held.back().marker,"closed source changed the caller output");
            if(session==0) {
                errno=0;
                require(fcntl(fd,F_GETFD)==-1 && errno==EBADF,"source initial socket not consumed/closed");
            }
            const size_t fds=count_entries("/proc/self/fd");
            const auto immediate=task_ids();
            bool settled=immediate==baseline;
            if(fds==base_fds && !settled && only_retiring_tasks(immediate,baseline,live)) {
                const auto label="cycle-"+std::to_string(cycle)+"-session-"+std::to_string(session);
                task_snapshot(artifacts,label+"-kernel-retirement",immediate);
                std::ofstream identities(artifacts/("tasks-"+label+"-live-identities.txt"));
                for(const auto& value:live) {
                    identities<<value.id<<' '<<value.name<<' '<<value.started<<' '<<value.flags<<' '<<value.address_space<<' '<<value.resident_pages<<'\n';
                }
                identities.close();require(identities.good(),"cannot archive reinit live identities");
                const auto start=std::chrono::steady_clock::now();
                for(;;) {
                    const auto current=task_ids();
                    if(current==baseline) { settled=true;break; }
                    if(!only_retiring_tasks(current,baseline,live) ||
                       std::chrono::steady_clock::now()-start>=std::chrono::milliseconds(100)) { break; }
                    std::this_thread::sleep_for(std::chrono::milliseconds(1));
                }
            }
            if(fds!=base_fds || !settled) {
                task_snapshot(artifacts,"cycle-"+std::to_string(cycle)+"-session-"+std::to_string(session)+"-mismatch",immediate);
            }
            std::cout<<"source reinit closed cycle="<<cycle<<" session="<<session<<" fds="<<fds
                <<" baseline_fds="<<base_fds<<" immediate_tasks="<<immediate.size()<<" settled="<<settled<<'\n';
            require(fds==base_fds && settled,"source reinit resource mismatch or unretired/unknown/live task");
            for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"reinit changed a held source image"); }
        }
        source.reset();
        require(count_entries("/proc/self/fd")==base_fds && task_ids()==baseline,"source reinit destructor resource mismatch");
    }
    std::exception_ptr error;
    std::thread reader([&] {
        try { for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"post-reinit cross-thread source image changed"); } }
        catch(...) { error=std::current_exception(); }
    });
    reader.join();if(error) { std::rethrow_exception(error); }
    if(inject_failure) {
        std::cout<<"original PipeWire fd-retry objects=20 injected_failures=20 proven_live_retries=20 sessions=40 captures=120 retained=120 consumed_sockets_closed=20 passed\n";
    } else {
        std::cout<<"original PipeWire reinit objects=20 sessions=40 captures=120 retained=120 consumed_sockets_closed=20 passed\n";
    }
}
}
#if defined(MFS_PW_KWIN_NATIVE_API)
#include "kwin-native-host.hpp"
#endif
int main(int argc,char** argv) {
    try {
        std::cout.setf(std::ios::unitbuf);
        verify_runtime();
#if defined(MFS_PW_KWIN_NATIVE_API)
        require(argc==5 && std::string_view(argv[1])=="--kwin-native","usage: --kwin-native socket node artifacts");
        uint32_t node=0;const std::string_view id=argv[3];
        const auto parsed=std::from_chars(id.data(),id.data()+id.size(),node);
        require(parsed.ec==std::errc{} && parsed.ptr==id.data()+id.size(),"invalid native KWin node");
        manager_kwin_native_context(argv[2],node,argv[4]);return 0;
#else
        if(argc==3 && std::string_view(argv[1])=="--manager-host-offer") {
            manager_host_offer_context(argv[2]);return 0;
        }
        if(argc==4 && std::string_view(argv[1])=="--sustained-manager") {
            uint32_t node=0;
            const std::string_view id=argv[2];
            const auto parsed=std::from_chars(id.data(),id.data()+id.size(),node);
            require(parsed.ec==std::errc{} && parsed.ptr==id.data()+id.size(),"invalid sustained manager node ID");
            sustained_manager_context(node,argv[3]);return 0;
        }
        if(argc==5 && (std::string_view(argv[1])=="--manager" || std::string_view(argv[1])=="--manager-fd")) {
            uint32_t node=0;
            const std::string_view id=argv[3];
            const auto parsed=std::from_chars(id.data(),id.data()+id.size(),node);
            require(parsed.ec==std::errc{} && parsed.ptr==id.data()+id.size(),"invalid explicit manager node ID");
            manager_context(argv[2],node,argv[4],std::string_view(argv[1])=="--manager-fd");
            return 0;
        }
        if(argc==5 && std::string_view(argv[1])=="--manager-host") {
            uint32_t node=0;
            const std::string_view id=argv[3];
            const auto parsed=std::from_chars(id.data(),id.data()+id.size(),node);
            require(parsed.ec==std::errc{} && parsed.ptr==id.data()+id.size(),"invalid host manager node ID");
            manager_host_context(argv[2],node,argv[4]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--pods") {
            save_pods(argv[2]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--format-fixtures") {
            save_format_fixtures(argv[2]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--memfd-fixtures") {
            save_memfd_fixtures(argv[2]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--dma-fixtures") {
            return save_dma_fixtures(argv[2]);
        }
        if(argc==2 && std::string_view(argv[1])=="--dma-error-control") {
            dma_error_control();return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--joined-task-observation") {
            observe_joined_tasks(argv[2]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--task-diagnostic-fixtures") {
            task_diagnostic_fixtures(argv[2]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--task-read-exit-fixture") {
            task_read_exit_fixture(argv[2]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--task-retirement-read-fixture") {
            task_retirement_read_fixture(argv[2]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--fd-error") {
            early_fd_error(argv[2]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--fd-add-io-error") {
            fd_add_io_error(argv[2]);
            return 0;
        }
        if(argc==5 && (std::string_view(argv[1])=="--reinit" || std::string_view(argv[1])=="--fd-retry")) {
            const std::string_view id=argv[3];
            uint32_t node=0;
            const auto parsed=std::from_chars(id.data(),id.data()+id.size(),node);
            require(parsed.ec==std::errc{} && parsed.ptr==id.data()+id.size(),"invalid explicit reinit node ID");
            reinitialization_context(argv[2],node,argv[4],std::string_view(argv[1])=="--fd-retry");
            return 0;
        }
        require(argc==4,"usage: source-context <private socket> <node ID> <artifact directory>");
        const std::string_view id=argv[2];
        uint32_t node=0;
        const auto parsed=std::from_chars(id.data(),id.data()+id.size(),node);
        require(parsed.ec==std::errc{} && parsed.ptr==id.data()+id.size(),"invalid explicit node ID");
        const std::filesystem::path artifacts=argv[3];
        std::filesystem::create_directories(artifacts);
        // Initialize conversion-only runtime outside capture resource accounting.
        cv::Mat raw(HEIGHT,WIDTH,CV_8UC3,cv::Scalar(1,2,3)),converted;
        cv::cvtColor(raw,converted,cv::COLOR_RGB2BGR);
        const size_t base_fds=count_entries("/proc/self/fd");
        const auto base_tasks=task_ids();
        const size_t base_threads=base_tasks.size();
        task_snapshot(artifacts,"baseline",base_tasks);
        std::vector<HeldImage> held;
        held.reserve(60);
        uint32_t previous=0;
        for(int cycle=0;cycle<20;++cycle) {
            std::cout<<"source begin cycle="<<cycle<<'\n';
            const int fd=connect_socket(argv[1]);
            // Transfer the descriptor at the original constructor boundary.
            // Its native connect call consumes it. No second close on error.
            Source* created=nullptr;
            try { created=new Source(fd,node); }
            catch(...) { close(fd);throw; }
            std::unique_ptr<Source,SourceOwner> source(created);
            require(source->init(),"original source init failed");
            require(source->connected(),"original source not connected after init");
            for(int call=0;call<3;++call) {
                std::cout<<"source begin cycle="<<cycle<<" call="<<call<<'\n';
                if(call==1) { source->inactive(); }
                cv::Mat image;
                const auto start=std::chrono::steady_clock::now();
                require(source->screencap(image),"original source screencap failed");
                const auto elapsed=std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::steady_clock::now()-start).count();
                std::cout<<"source observation cycle="<<cycle<<" call="<<call<<" width="<<image.cols
                    <<" height="<<image.rows<<" type="<<image.type()<<" stride="<<image.step[0]<<'\n';
                if(image.type()==CV_8UC3 && image.cols==WIDTH && image.rows==HEIGHT) {
                    save(image,artifacts/("cycle-"+std::to_string(cycle)+"-call-"+std::to_string(call)+".bgr"));
                }
                const uint32_t marker=known_pixels(image);
                const char* relation=previous==0 ? "first" : marker==previous ? "repeat" : marker>previous ? "advance" : "regress";
                held.push_back({std::move(image),marker});
                std::cout<<"source cycle="<<cycle<<" call="<<call<<" elapsed_us="<<elapsed
                    <<" output=known_bgr fixture_marker="<<marker<<" marker_relation="<<relation<<" api_replay=not_exposed\n";
                previous=marker;
            }
            const auto live=live_tasks();
            if(cycle==0) { task_snapshot(artifacts,"first-live",task_ids()); }
            source->close();
            require(!source->connected(),"original source still connected after close");
            source.reset();
            errno=0;
            require(fcntl(fd,F_GETFD)==-1 && errno==EBADF,"source input socket still open after destruction");
            const size_t fds=count_entries("/proc/self/fd");
            const auto tasks=task_ids();
            const size_t threads=tasks.size();
            std::cout<<"source closed cycle="<<cycle<<" fds="<<fds<<" baseline_fds="<<base_fds
                <<" threads="<<threads<<" baseline_threads="<<base_threads<<'\n';
            bool settled=tasks==base_tasks;
            if(fds==base_fds && !settled && only_retiring_tasks(tasks,base_tasks,live)) {
                const auto label="cycle-"+std::to_string(cycle);
                task_snapshot(artifacts,label+"-kernel-retirement",tasks);
                std::ofstream identities(artifacts/("tasks-"+label+"-live-identities.txt"));
                for(const auto& value:live) {
                    identities<<value.id<<' '<<value.name<<' '<<value.started<<' '<<value.flags<<' '<<value.address_space<<' '<<value.resident_pages<<'\n';
                }
                identities.close();
                require(identities.good(),"cannot archive live task identities");
                const auto start=std::chrono::steady_clock::now();
                // Full source close/destruction already finished. This is a
                // bounded kernel/proc retirement observation, never a capture
                // retry, wait for active callbacks, or deferred FD cleanup.
                for(;;) {
                    const auto current=task_ids();
                    if(current==base_tasks) { settled=true;break; }
                    if(!only_retiring_tasks(current,base_tasks,live) ||
                       std::chrono::steady_clock::now()-start>=std::chrono::milliseconds(100)) { break; }
                    std::this_thread::sleep_for(std::chrono::milliseconds(1));
                }
                std::cout<<"kernel task retirement cycle="<<cycle<<" immediate_tasks="<<threads<<" baseline="<<base_threads
                    <<" settled="<<settled<<" elapsed_us="
                    <<std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::steady_clock::now()-start).count()<<'\n';
            }
            if(fds!=base_fds || !settled) {
                std::cerr<<"source native resource mismatch fixed before diagnostic observations\n";
                const auto label="cycle-"+std::to_string(cycle);
                task_snapshot(artifacts,label+"-mismatch",tasks);
                for(int delay:{1,10,100}) {
                    std::this_thread::sleep_for(std::chrono::milliseconds(delay));
                    task_snapshot(artifacts,label+"-after-"+std::to_string(delay)+"ms",task_ids());
                }
            }
            require(fds==base_fds && settled,"source native resource count mismatch or unretired/unknown/live task");
            for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"retained source sample identity changed"); }
        }
        std::exception_ptr cross_thread_error;
        std::thread reader([&] {
            try { for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"cross-thread source sample identity changed"); } }
            catch(...) { cross_thread_error=std::current_exception(); }
        });
        reader.join();
        if(cross_thread_error) { std::rethrow_exception(cross_thread_error); }
        save(held.front().image,artifacts/"first-after-shutdown.bgr");
        save(held.back().image,artifacts/"last-after-shutdown.bgr");
        std::cout<<"original PipeWire source cycles=20 captures=60 retained=60 passed\n";
        return 0;
#endif
    } catch(const std::exception& error) {
        std::cerr<<"source context failure: "<<error.what()<<'\n';
        return 1;
    } catch(...) {
        std::cerr<<"source context unknown exception\n";
        return 2;
    }
}
