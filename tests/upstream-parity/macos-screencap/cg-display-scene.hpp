// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Separate producer control. No SCShareableContent/capture call is made here.
struct CgSceneUnavailable : std::runtime_error {using std::runtime_error::runtime_error;};
DisplayGeometry cg_scene_geometry(uint32_t wid,uint64_t& queries) {
    require([NSThread isMainThread],"CoreGraphics scene query is not on main");
    uint32_t count=0;
    require(CGGetActiveDisplayList(0,nullptr,&count)==kCGErrorSuccess,"cannot count active scene displays");
    if(count!=1) {throw CgSceneUnavailable("CoreGraphics scene control requires one active display");}
    std::array<CGDirectDisplayID,2> displays={};uint32_t written=0;
    require(CGGetActiveDisplayList(static_cast<uint32_t>(displays.size()),displays.data(),&written)==kCGErrorSuccess,
        "cannot query active scene display");
    if(written!=1) {throw CgSceneUnavailable("active display count changed during scene query");}
    const auto id=displays[0];require(id!=kCGNullDirectDisplay,"scene display ID missing");
    const auto pixels_wide=CGDisplayPixelsWide(id),pixels_high=CGDisplayPixelsHigh(id);
    require(pixels_wide>0 && pixels_wide<=INT_MAX && pixels_high>0 && pixels_high<=INT_MAX,"scene display dimensions overflow");
    DisplayGeometry g;g.count=written;g.id=id;g.width=static_cast<int>(pixels_wide);g.height=static_cast<int>(pixels_high);
    g.display=CGDisplayBounds(id);
    for(double edge:{g.display.origin.x,g.display.origin.y,g.display.size.width,g.display.size.height}) {
        require(std::isfinite(edge),"scene display coordinate is nonfinite");
    }
    require(g.display.size.width>0 && g.display.size.height>0,"scene display bounds empty");
    if(wid) {
        struct ArrayOwner {
            CFArrayRef value;
            ~ArrayOwner() {if(value) {CFRelease(value);}}
        } windows{CGWindowListCopyWindowInfo(kCGWindowListOptionIncludingWindow,wid)};
        require(windows.value!=nullptr && CFArrayGetCount(windows.value)==1,"owned scene window description missing/ambiguous");
        const auto raw=CFArrayGetValueAtIndex(windows.value,0);
        require(raw && CFGetTypeID(raw)==CFDictionaryGetTypeID(),"scene window description type differs");
        const auto dictionary=static_cast<CFDictionaryRef>(raw);
        const auto integer=[&](CFStringRef key) {
            const auto value=CFDictionaryGetValue(dictionary,key);int64_t output=0;
            require(value && CFGetTypeID(value)==CFNumberGetTypeID() &&
                CFNumberGetValue(static_cast<CFNumberRef>(value),kCFNumberSInt64Type,&output),"scene window integer missing/invalid");
            return output;
        };
        require(integer(kCGWindowNumber)==wid,"CoreGraphics substituted scene window");
        const auto pid=integer(kCGWindowOwnerPID);require(pid>0 && pid==getpid(),"scene window has a different process owner");
        const auto bounds=CFDictionaryGetValue(dictionary,kCGWindowBounds);
        require(bounds && CFGetTypeID(bounds)==CFDictionaryGetTypeID() &&
            CGRectMakeWithDictionaryRepresentation(static_cast<CFDictionaryRef>(bounds),&g.bounds),"scene window bounds missing/invalid");
        const auto visible=CFDictionaryGetValue(dictionary,kCGWindowIsOnscreen);
        require(visible && CFGetTypeID(visible)==CFBooleanGetTypeID() && CFBooleanGetValue(static_cast<CFBooleanRef>(visible)),
            "owned scene window is not onscreen");
        g.window=wid;g.pid=static_cast<uint64_t>(pid);g.visible=true;
    }
    require(queries<std::numeric_limits<uint64_t>::max(),"scene query counter overflow");++queries;return g;
}
void display_patch_cg_scene(NSApplication* app,const std::filesystem::path& directory) {
    uint64_t queries=0;DisplayGeometry geometry;
    {
        Window window(app,0);const auto first=cg_scene_geometry(0,queries);NSScreen* selected=nil;
        for(NSScreen* screen in [NSScreen screens]) {
            NSNumber* number=[[screen deviceDescription] objectForKey:@"NSScreenNumber"];
            if(number && [number unsignedLongLongValue]==first.id) {selected=screen;break;}
        }
        require(selected!=nil,"active CoreGraphics display has no matching AppKit screen");const auto screen=[selected frame];
        [window.value setFrameOrigin:NSMakePoint(std::floor(NSMidX(screen)-width/2.0),std::floor(NSMidY(screen)-height/2.0))];
        [window.value setLevel:NSFloatingWindowLevel];[window.value orderFrontRegardless];window.paint(app,0);
        geometry=cg_scene_geometry(window.id(),queries);require(geometry.id==first.id,"CoreGraphics display changed during placement");
        (void)display_region(geometry);
        std::ofstream metadata(directory/"display-geometry.tsv",std::ios::binary);metadata<<display_geometry_text(geometry);
        metadata.close();require(metadata.good(),"cannot publish CoreGraphics scene geometry");
        std::ofstream ready(directory/"ready.tmp");
        ready<<"{\"window_id\":"<<window.id()<<",\"width\":47,\"height\":31,\"colour\":\"green\",\"owner_pid\":"<<getpid()<<",\"display_patch\":1}\n";
        ready.close();require(ready.good(),"cannot publish CoreGraphics scene identity");std::filesystem::rename(directory/"ready.tmp",directory/"ready.json");
        const auto until=Clock::now()+std::chrono::minutes(3);
        while(!std::filesystem::exists(directory/"stop")) {require(Clock::now()<until,"CoreGraphics scene exceeded lifetime budget");pump();}
    }
    std::ofstream state(directory/"cg-scene-state.txt",std::ios::binary);
    state<<"geometry_origin=core_graphics\ngeometry_queries="<<queries<<"\ndisplay_count="<<geometry.count
         <<"\ndisplay_id="<<geometry.id<<"\nscene_pid="<<geometry.pid<<"\nwindow_id="<<geometry.window
         <<"\nmain_thread="<<[NSThread isMainThread]<<"\nwindow_closed=1\n";
    state.close();require(state.good() && queries==2,"cannot record CoreGraphics scene closure");
    std::ofstream stopped(directory/"stopped.tmp");stopped<<"window_closed=1\n";stopped.close();
    require(stopped.good(),"cannot acknowledge CoreGraphics scene closure");std::filesystem::rename(directory/"stopped.tmp",directory/"stopped.txt");
    std::cout<<"owned macOS cg_scene geometry_queries=2 single_active_display=1 window_closed=1 passed\n"
             <<"owned macOS scene stop=1 main_thread=1 passed\n";
}
