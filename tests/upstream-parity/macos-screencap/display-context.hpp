// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included in the explicit AppKit source fixture, never the capture backend.
struct DisplayGeometry {
    uint64_t count=0,id=0,window=0,pid=0;
    int width=0,height=0;
    CGRect display={},bounds={};
    bool visible=false;
};
DisplayGeometry display_geometry(uint32_t wid) {
    struct Completion {DisplayGeometry value;std::exception_ptr error;std::atomic<bool> done=false;};
    auto state=std::make_shared<Completion>();
    [SCShareableContent getShareableContentWithCompletionHandler:^(SCShareableContent* content,NSError* error) {
        try {
            if(error || !content || content.displays.count==0) {throw std::runtime_error("fixture cannot observe first display");}
            SCDisplay* first=content.displays[0];
            require(first.width>0 && first.width<=INT_MAX && first.height>0 && first.height<=INT_MAX,"fixture display dimensions overflow");
            auto& value=state->value;value.count=content.displays.count;value.id=first.displayID;
            value.width=static_cast<int>(first.width);value.height=static_cast<int>(first.height);value.display=first.frame;
            if(wid!=0) {
                for(SCWindow* window in content.windows) {if(window.windowID==wid) {
                    value.window=wid;value.bounds=window.frame;value.visible=window.onScreen;
                    const auto pid=window.owningApplication.processID;
                    require(pid>0,"display patch has no owning native process");value.pid=static_cast<uint64_t>(pid);break;
                }}
                require(value.window==wid && value.visible,"display patch window missing/offscreen");
            }
        }catch(...) {state->error=std::current_exception();}
        state->done.store(true,std::memory_order_release);
    }];
    const auto until=Clock::now()+std::chrono::seconds(8);
    while(!state->done.load(std::memory_order_acquire)) {require(Clock::now()<until,"fixture display observation timed out");pump();}
    if(state->error) {std::rethrow_exception(state->error);}return state->value;
}
std::string display_geometry_text(const DisplayGeometry& g) {
    std::ostringstream out;out<<std::setprecision(std::numeric_limits<double>::max_digits10);
    out<<g.count<<'\t'<<g.id<<'\t'<<g.width<<'\t'<<g.height<<'\t'
       <<g.display.origin.x<<'\t'<<g.display.origin.y<<'\t'<<g.display.size.width<<'\t'<<g.display.size.height<<'\t'
       <<g.window<<'\t'<<g.pid<<'\t'<<g.bounds.origin.x<<'\t'<<g.bounds.origin.y<<'\t'
       <<g.bounds.size.width<<'\t'<<g.bounds.size.height<<'\t'<<g.visible<<"\t2\n";return out.str();
}
struct DisplayRegion {int left=0,top=0,right=0,bottom=0;};
DisplayRegion display_region(const DisplayGeometry& g) {
    require(g.width>0 && g.height>0 && g.display.size.width>0 && g.display.size.height>0
        && g.bounds.size.width==width && g.bounds.size.height==height,"display patch point geometry differs");
    const double sx=g.width/g.display.size.width,sy=g.height/g.display.size.height;
    const std::array<double,4> edges={
        std::ceil((g.bounds.origin.x+2-g.display.origin.x)*sx),std::ceil((g.bounds.origin.y+2-g.display.origin.y)*sy),
        std::floor((g.bounds.origin.x+g.bounds.size.width-2-g.display.origin.x)*sx),
        std::floor((g.bounds.origin.y+g.bounds.size.height-2-g.display.origin.y)*sy)};
    for(double edge:edges) {require(std::isfinite(edge) && edge>=0 && edge<=INT_MAX,"display patch edge overflow/outside source");}
    DisplayRegion r{static_cast<int>(edges[0]),static_cast<int>(edges[1]),static_cast<int>(edges[2]),static_cast<int>(edges[3])};
    require(r.left<r.right && r.top<r.bottom && r.right<=g.width && r.bottom<=g.height,"display patch outside first source");return r;
}
void display_pixels(const cv::Mat& image,const DisplayGeometry& g) {
    require(image.type()==CV_8UC3 && image.cols==g.width && image.rows==g.height,"original display output dimensions differ from first SCDisplay");
    const auto r=display_region(g);
    for(int y=r.top;y<r.bottom;++y) {for(int x=r.left;x<r.right;++x) {
        require(image.at<cv::Vec3b>(y,x)==cv::Vec3b(0,255,0),"first-display owned green patch pixel differs");
    }}
}
void display_patch_scene(NSApplication* app,const std::filesystem::path& directory) {
    {
        Window window(app,0);const auto first=display_geometry(0);NSScreen* selected=nil;
        for(NSScreen* screen in [NSScreen screens]) {
            NSNumber* number=[[screen deviceDescription] objectForKey:@"NSScreenNumber"];
            if(number && [number unsignedLongLongValue]==first.id) {selected=screen;break;}
        }
        require(selected!=nil,"first enumerated SCDisplay has no matching AppKit screen");const auto screen=[selected frame];
        [window.value setFrameOrigin:NSMakePoint(std::floor(NSMidX(screen)-width/2.0),std::floor(NSMidY(screen)-height/2.0))];
        [window.value setLevel:NSFloatingWindowLevel];[window.value orderFrontRegardless];window.paint(app,0);
        const auto g=display_geometry(window.id());require(g.id==first.id,"first display changed during fixture placement");(void)display_region(g);
        std::ofstream geometry(directory/"display-geometry.tsv",std::ios::binary);geometry<<display_geometry_text(g);
        geometry.close();require(geometry.good(),"cannot publish native display/owned-window geometry");
        std::ofstream ready(directory/"ready.tmp");
        ready<<"{\"window_id\":"<<window.id()<<",\"width\":47,\"height\":31,\"colour\":\"green\",\"owner_pid\":"<<getpid()<<",\"display_patch\":1}\n";
        ready.close();require(ready.good(),"cannot publish display scene identity");std::filesystem::rename(directory/"ready.tmp",directory/"ready.json");
        const auto until=Clock::now()+std::chrono::minutes(3);
        while(!std::filesystem::exists(directory/"stop")) {require(Clock::now()<until,"display scene exceeded lifetime budget");pump();}
    }
    std::ofstream stopped(directory/"stopped.tmp");stopped<<"window_closed=1\n";stopped.close();
    require(stopped.good(),"cannot acknowledge display scene closure");std::filesystem::rename(directory/"stopped.tmp",directory/"stopped.txt");
    std::cout<<"owned macOS scene stop=1 main_thread=1 passed\n";
}
void display_patch(const std::filesystem::path& directory,const char* argument) {
    const auto wid=std::stoull(argument);require(wid>0 && wid<=UINT32_MAX,"display patch window ID narrowing");
    const auto g=display_geometry(static_cast<uint32_t>(wid));const auto identity=display_geometry_text(g);
    std::ofstream metadata(directory/"source-display-geometry.tsv",std::ios::binary);metadata<<identity;
    metadata.close();require(metadata.good(),"cannot save actual original display target evidence");
    std::vector<cv::Mat> held;held.reserve(3);
    {
        MfsOriginalMac::ScreenCaptureKitScreencap source(0);
        for(unsigned call=0;call<3;++call) {
            require(display_geometry_text(display_geometry(static_cast<uint32_t>(wid)))==identity,"source target geometry changed before capture");
            auto image=capture(source);require(image.has_value(),"original display-patch capture returned nullopt");
            display_pixels(*image,g);save(*image,directory/("display-call-"+std::to_string(call)+".source.bgr"));held.push_back(std::move(*image));
        }
    }
    require(display_geometry_text(display_geometry(static_cast<uint32_t>(wid)))==identity,"source target geometry changed after capture");
    for(size_t call=0;call<held.size();++call) {display_pixels(held[call],g);save(held[call],directory/("display-held-"+std::to_string(call)+".source.bgr"));}
    std::cout<<"original macOS display_patch first_enumerated=1 captures=3 retained=3 native_geometry=1 known_owned_patch=3 full_display_pixels_unasserted=1 passed\n";
}
