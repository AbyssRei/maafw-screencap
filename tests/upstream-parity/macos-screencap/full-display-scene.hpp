// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// A separate all-pixel control. Existing small-patch scenes stay unchanged.
void full_display_cg_scene(NSApplication* app,const std::filesystem::path& directory) {
    uint64_t queries=0;DisplayGeometry geometry;int64_t window_level=0;
    {
        const auto first=cg_scene_geometry(0,queries);NSScreen* selected=nil;
        for(NSScreen* screen in [NSScreen screens]) {
            NSNumber* number=[[screen deviceDescription] objectForKey:@"NSScreenNumber"];
            if(number && [number unsignedLongLongValue]==first.id) {selected=screen;break;}
        }
        if(!selected || [selected backingScaleFactor]!=1.0 || first.width<3 || first.height<2 ||
            first.display.size.width!=first.width || first.display.size.height!=first.height ||
            [selected frame].size.width!=first.width || [selected frame].size.height!=first.height) {
            throw CgSceneUnavailable("full-display RGB control requires single-display 1:1 native geometry");
        }
        Window window(app,0);
        [window.value setLevel:CGWindowLevelForKey(kCGScreenSaverWindowLevelKey)];
        [window.value setFrame:[selected frame] display:YES];
        window_level=[window.value level];require(window_level>0,"full-display level unavailable");
        const CGFloat wide=first.width,high=first.height,cut=first.width/3,middle=first.height/2;
        struct ImageOwner {NSImage* value;~ImageOwner(){[value release];}} image{
            [[NSImage imageWithSize:NSMakeSize(wide,high) flipped:NO drawingHandler:^BOOL(NSRect rectangle) {
                (void)rectangle;
                // Only scalar dimensions escape in this copied block. AppKit
                // owns the image/view; no borrowed C++ stack/window pointer.
                [[NSGraphicsContext currentContext] setShouldAntialias:NO];
                [[NSColor colorWithSRGBRed:0 green:0 blue:1 alpha:1] set];
                NSRectFill(NSMakeRect(0,0,wide,high));
                [[NSColor colorWithSRGBRed:1 green:0 blue:0 alpha:1] set];
                NSRectFill(NSMakeRect(0,0,cut,high));
                [[NSColor colorWithSRGBRed:0 green:1 blue:0 alpha:1] set];
                NSRectFill(NSMakeRect(cut,middle,wide-cut,high-middle));
                return YES;
            }] retain]}; // Factory is +0; this fixture balances its explicit +1.
        require(image.value!=nil,"full-display image allocation failed");
        struct ViewOwner {NSImageView* value;~ViewOwner(){[value release];}} view{
            [[NSImageView alloc] initWithFrame:NSMakeRect(0,0,wide,high)]};
        require(view.value!=nil,"full-display view allocation failed");
        [view.value setImage:image.value];[view.value setImageScaling:NSImageScaleNone];
        [view.value setImageFrameStyle:NSImageFrameNone];[window.value setContentView:view.value];
        [window.value orderFrontRegardless];[window.value display];[app updateWindows];
        // Fixture presentation before either capture implementation is called.
        const auto presented=Clock::now()+std::chrono::milliseconds(100);
        while(Clock::now()<presented) {pump();}
        geometry=cg_scene_geometry(window.id(),queries);
        require(geometry.id==first.id && CGRectEqualToRect(geometry.bounds,geometry.display),
            "full-display window does not cover exactly the active display");
        auto text=display_geometry_text(geometry);
        require(text.size()>=3 && text.substr(text.size()-3)=="\t2\n","unexpected geometry suffix");
        text.replace(text.size()-3,3,"\t0\n"); // tested inset=0: every output pixel
        std::ofstream metadata(directory/"display-geometry.tsv",std::ios::binary);metadata<<text;
        metadata.close();require(metadata.good(),"cannot publish full-display geometry");
        std::ofstream ready(directory/"ready.tmp");
        ready<<"{\"window_id\":"<<window.id()<<",\"width\":"<<first.width<<",\"height\":"<<first.height
             <<",\"owner_pid\":"<<getpid()<<",\"full_display_rgb\":1}\n";
        ready.close();require(ready.good(),"cannot publish full-display ready marker");
        std::filesystem::rename(directory/"ready.tmp",directory/"ready.json");
        const auto until=Clock::now()+std::chrono::minutes(3);
        while(!std::filesystem::exists(directory/"stop")) {
            require(Clock::now()<until,"full-display scene exceeded lifetime budget");pump();
        }
    }
    std::ofstream state(directory/"cg-scene-state.txt",std::ios::binary);
    state<<"geometry_origin=core_graphics\ngeometry_queries="<<queries<<"\ndisplay_count="<<geometry.count
         <<"\ndisplay_id="<<geometry.id<<"\nscene_pid="<<geometry.pid<<"\nwindow_id="<<geometry.window
         <<"\nmain_thread="<<[NSThread isMainThread]<<"\nwindow_closed=1\nfull_display_rgb=1\npixel_inset=0\nbacking_scale=1\nwindow_level="<<window_level<<'\n';
    state.close();require(state.good() && queries==2,"cannot record full-display scene closure");
    std::ofstream stopped(directory/"stopped.tmp");stopped<<"window_closed=1\n";stopped.close();
    require(stopped.good(),"cannot acknowledge full-display window closure");
    std::filesystem::rename(directory/"stopped.tmp",directory/"stopped.txt");
    std::cout<<"owned macOS full_display_rgb=1 inset=0 single_display_1to1=1 window_closed=1 passed\n"
             <<"owned macOS scene stop=1 main_thread=1 passed\n";
}
