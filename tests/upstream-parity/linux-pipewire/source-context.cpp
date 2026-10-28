// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Compile the complete unedited original translation unit here, so its static
// negotiation helpers are available to the explicit POD oracle below.
#include "Screencap/PipeWireScreencap.cpp"
#include <pipewire/pipewire.h>
#include <opencv2/imgproc.hpp>
#include <algorithm>
#include <array>
#include <cerrno>
#include <charconv>
#include <chrono>
#include <cstring>
#include <cstdlib>
#include <exception>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <limits>
#include <memory>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string_view>
#include <thread>
#include <vector>
#include <fcntl.h>
#include <dlfcn.h>
#include <pthread.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

namespace {
using Source=MfsOriginalPipeWire::PipeWireScreencap;
constexpr int WIDTH=28,HEIGHT=24;
void require(bool condition,const char* message) {
    if(!condition) { throw std::runtime_error(message); }
}
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
std::optional<Task> task(const std::string& id) {
    errno=0;
    std::ifstream input(std::filesystem::path("/proc/self/task")/id/"stat");
    if(!input) { require(errno==ENOENT,"cannot inspect proc task");return std::nullopt; }
    std::string text;
    std::getline(input,text);
    require(input.good() || input.eof(),"cannot read proc task stat");
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
    for(const auto& id:baseline) {
        if(std::find(actual.begin(),actual.end(),id)==actual.end()) { return false; }
    }
    for(const auto& id:actual) {
        if(std::find(baseline.begin(),baseline.end(),id)!=baseline.end()) { continue; }
        const auto before=std::find_if(live.begin(),live.end(),[&](const Task& value){return value.id==id;});
        if(before==live.end() || before->name!="MaaScreencap" ||
           (before->flags&0x4) || before->address_space==0) { return false; }
        if(auto after=task(id)) {
            // Only a known same-generation PF_EXITING task with no userspace
            // address space can finish kernel retirement here. Never a live
            // worker, an unknown TID, or asynchronous callback/native cleanup.
            if(after->started!=before->started || !(after->flags&0x4) ||
               after->address_space!=0 || after->resident_pages!=0) { return false; }
        }
    }
    return true;
}
void task_snapshot(const std::filesystem::path& directory,const std::string& label,const std::vector<std::string>& tasks) {
    std::ofstream output(directory/("tasks-"+label+".txt"));
    require(output.good(),"cannot write task diagnostic");
    output<<"tasks observed="<<tasks.size()<<'\n';
    for(const auto& tid:tasks) {
        for(const char* name:{"comm","stat","status","wchan"}) {
            const auto path=std::filesystem::path("/proc/self/task")/tid/name;
            std::ifstream input(path);
            output<<path<<" open="<<input.good()<<'\n';
            if(input) { output<<input.rdbuf()<<'\n'; }
        }
    }
    output.close();
    require(output.good(),"cannot close task diagnostic");
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
}
int main(int argc,char** argv) {
    try {
        std::cout.setf(std::ios::unitbuf);
        verify_runtime();
        if(argc==3 && std::string_view(argv[1])=="--pods") {
            save_pods(argv[2]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--format-fixtures") {
            save_format_fixtures(argv[2]);
            return 0;
        }
        if(argc==3 && std::string_view(argv[1])=="--joined-task-observation") {
            observe_joined_tasks(argv[2]);
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
    } catch(const std::exception& error) {
        std::cerr<<"source context failure: "<<error.what()<<'\n';
        return 1;
    } catch(...) {
        std::cerr<<"source context unknown exception\n";
        return 2;
    }
}
