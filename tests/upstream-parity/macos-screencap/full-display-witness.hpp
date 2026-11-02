// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Separate fixture observations; never a display-capture or parity oracle.
#include <utility>
void full_witness_json(const std::filesystem::path& path,id value) {
    NSError* error=nil;
    NSData* data=[NSJSONSerialization dataWithJSONObject:value options:0 error:&error];
    require(data && !error && data.length<=static_cast<NSUInteger>(std::numeric_limits<std::streamsize>::max()),
        "cannot serialize full-display witness");
    std::ofstream out(path,std::ios::binary);
    out.write(static_cast<const char*>(data.bytes),static_cast<std::streamsize>(data.length));
    out.close();require(out.good(),"cannot save full-display witness JSON");
}
void full_display_witness(NSView* view,const DisplayGeometry& geometry,int64_t level,
    uint32_t slot,const std::filesystem::path& directory) {
    require([NSThread isMainThread] && slot<7,"full-display witness thread/index differs");
    require(geometry.width>=3 && geometry.height>=2,"full-display witness dimensions invalid");
    @autoreleasepool {
        const auto bounds=[view bounds];
        // AppKit draws this owned view into CPU storage. This does not sample
        // WindowServer composition or invoke another screenshot operation.
        struct BitmapOwner {NSBitmapImageRep* value;~BitmapOwner(){[value release];}} factory{
            [[view bitmapImageRepForCachingDisplayInRect:bounds] retain]};
        require(factory.value!=nil,"view CPU bitmap allocation unavailable");
        // Configure the empty destination before drawing. This is not a retag
        // or reinterpretation of any previously captured/rendered image.
        BitmapOwner bitmap{[[factory.value bitmapImageRepByRetaggingWithColorSpace:[NSColorSpace sRGBColorSpace]] retain]};
        require(bitmap.value && [[bitmap.value colorSpace] isEqual:[NSColorSpace sRGBColorSpace]],
            "view CPU destination is not explicitly sRGB");
        require([bitmap.value pixelsWide]==geometry.width && [bitmap.value pixelsHigh]==geometry.height,
            "view CPU bitmap does not have the required 1:1 dimensions");
        const auto wide=static_cast<size_t>(geometry.width),high=static_cast<size_t>(geometry.height);
        require(wide<=std::numeric_limits<size_t>::max()/4 && high<=std::numeric_limits<size_t>::max()/(wide*4),
            "view witness byte span overflow");
        require([bitmap.value bitsPerSample]==8 && [bitmap.value bitsPerPixel]==32 &&
            [bitmap.value samplesPerPixel]==4 && [bitmap.value hasAlpha] && ![bitmap.value isPlanar] &&
            [bitmap.value bitmapFormat]==0,
            "view CPU destination requires interleaved 8-bit RGBA storage");
        const auto native_stride=[bitmap.value bytesPerRow],native_plane=[bitmap.value bytesPerPlane];
        require(native_stride>0 && native_plane>0 && static_cast<size_t>(native_stride)>=wide*4 &&
            static_cast<size_t>(native_stride)<=std::numeric_limits<size_t>::max()/high,
            "view CPU destination span overflow");
        const auto native_span=static_cast<size_t>(native_stride)*high;
        require(native_span<=static_cast<size_t>(native_plane) && [bitmap.value bitmapData]!=nullptr,
            "view CPU destination storage missing");
        std::fill_n([bitmap.value bitmapData],native_span,0);
        [view cacheDisplayInRect:bounds toBitmapImageRep:bitmap.value];
        require([[bitmap.value colorSpace] isEqual:[NSColorSpace sRGBColorSpace]],
            "view redraw changed the explicitly prepared sRGB destination");
        const auto span=wide*4*high;
        require(span<=static_cast<size_t>(std::numeric_limits<std::streamsize>::max()),"view witness write narrowing");
        std::vector<unsigned char> bytes(span);
        for(int y=0;y<geometry.height;++y) {
            @autoreleasepool {
                for(int x=0;x<geometry.width;++x) {
                    // getPixel uses a top-left origin and the validated bitmap
                    // component order: format0, eight-bit RGBA, alpha last.
                    // The destination was configured as sRGB before drawing;
                    // do not convert or retag completed raster values here.
                    NSUInteger pixel[4]={};[bitmap.value getPixel:pixel atX:x y:y];
                    for(const auto value:pixel) {require(value<=255,"view component narrowing");}
                    const auto offset=(static_cast<size_t>(y)*wide+static_cast<size_t>(x))*4;
                    bytes[offset]=static_cast<unsigned char>(pixel[2]);bytes[offset+1]=static_cast<unsigned char>(pixel[1]);
                    bytes[offset+2]=static_cast<unsigned char>(pixel[0]);bytes[offset+3]=static_cast<unsigned char>(pixel[3]);
                }
            }
        }
        const auto suffix=std::to_string(slot);
        std::ofstream out(directory/("view-"+suffix+".bgra"),std::ios::binary);
        out.write(reinterpret_cast<const char*>(bytes.data()),static_cast<std::streamsize>(span));
        out.close();require(out.good(),"cannot save view CPU witness");
        full_witness_json(directory/("view-"+suffix+".json"),@{
            @"schema":@2,@"slot":@(slot),@"pid":@(getpid()),@"main_thread":@1,
            @"destination_srgb":@1,@"prepared_before_draw":@1,@"source_initialized":@1,
            @"window_id":@(geometry.window),@"width":@(geometry.width),@"height":@(geometry.height),
            @"source_bits_per_sample":@([bitmap.value bitsPerSample]),
            @"source_bits_per_pixel":@([bitmap.value bitsPerPixel]),
            @"source_samples_per_pixel":@([bitmap.value samplesPerPixel]),
            @"source_bytes_per_row":@([bitmap.value bytesPerRow]),
            @"source_bitmap_format":@(static_cast<uint64_t>([bitmap.value bitmapFormat])),
            @"source_has_alpha":@([bitmap.value hasAlpha]!=NO),@"source_planar":@([bitmap.value isPlanar]!=NO),
            @"canonical_format":@"BGRA8_sRGB",@"canonical_stride":@(wide*4),
            @"top_left_origin":@1,@"origin":@"NSView_cacheDisplay_sRGB_getPixel"});

        struct ArrayOwner {CFArrayRef value;~ArrayOwner(){if(value){CFRelease(value);}}} windows{
            CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly,kCGNullWindowID)};
        require(windows.value!=nullptr,"cannot enumerate onscreen window metadata");
        NSMutableArray* observed=[NSMutableArray array];
        const auto count=CFArrayGetCount(windows.value);bool found=false;
        for(CFIndex index=0;index<count;++index) {
            const auto raw=CFArrayGetValueAtIndex(windows.value,index);
            require(raw && CFGetTypeID(raw)==CFDictionaryGetTypeID(),"window census element type differs");
            const auto dictionary=static_cast<CFDictionaryRef>(raw);
            const auto integer=[&](CFStringRef key) {
                const auto value=CFDictionaryGetValue(dictionary,key);int64_t result=0;
                require(value && CFGetTypeID(value)==CFNumberGetTypeID() &&
                    CFNumberGetValue(static_cast<CFNumberRef>(value),kCFNumberSInt64Type,&result),"window census integer missing");
                return result;
            };
            const auto number=integer(kCGWindowNumber),pid=integer(kCGWindowOwnerPID),layer=integer(kCGWindowLayer);
            const auto box=CFDictionaryGetValue(dictionary,kCGWindowBounds);CGRect rectangle={};
            require(box && CFGetTypeID(box)==CFDictionaryGetTypeID() &&
                CGRectMakeWithDictionaryRepresentation(static_cast<CFDictionaryRef>(box),&rectangle),"window census bounds missing");
            for(double edge:{rectangle.origin.x,rectangle.origin.y,rectangle.size.width,rectangle.size.height}) {
                require(std::isfinite(edge),"window census coordinate nonfinite");
            }
            const auto visible=CFDictionaryGetValue(dictionary,kCGWindowIsOnscreen);
            // Apple defines IsOnscreen as an optional dictionary key. Preserve
            // its absence; this separate query explicitly selects onscreen windows.
            id visibility=[NSNull null];
            if(visible) {
                require(CFGetTypeID(visible)==CFBooleanGetTypeID(),"window census visibility type differs");
                visibility=@(CFBooleanGetValue(static_cast<CFBooleanRef>(visible))!=0);
            }
            NSMutableDictionary* row=[NSMutableDictionary dictionaryWithDictionary:@{
                @"rank":@(index),@"id":@(number),@"pid":@(pid),@"layer":@(layer),
                @"onscreen":visibility,
                @"bounds":@[@(rectangle.origin.x),@(rectangle.origin.y),@(rectangle.size.width),@(rectangle.size.height)]}];
            for(const auto pair:{std::pair<CFStringRef,NSString*>{kCGWindowOwnerName,@"owner_name"},
                std::pair<CFStringRef,NSString*>{kCGWindowName,@"name"}}) {
                const auto value=CFDictionaryGetValue(dictionary,pair.first);
                if(value) {
                    require(CFGetTypeID(value)==CFStringGetTypeID(),"window census name type differs");
                    [row setObject:(NSString*)value forKey:pair.second];
                }
            }
            [observed addObject:row];
            if(number==static_cast<int64_t>(geometry.window)) {
                require(pid==getpid() && layer==level && CGRectEqualToRect(rectangle,geometry.bounds) &&
                    (!visible || CFBooleanGetValue(static_cast<CFBooleanRef>(visible))),"window census substituted fixture owner/bounds/level");
                require(!found,"window census repeated owned target");found=true;
            }
        }
        require(found,"owned full-display window missing from census");
        full_witness_json(directory/("census-"+suffix+".json"),@{
            @"schema":@2,@"slot":@(slot),@"pid":@(getpid()),@"main_thread":@1,
            @"selection":@"kCGWindowListOptionOnScreenOnly",
            @"window_id":@(geometry.window),@"display_id":@(geometry.id),@"windows":observed});
        std::ofstream ready(directory/("witness-"+suffix+".tmp"),std::ios::binary);
        ready<<"slot="<<slot<<"\nmain_thread=1\n";ready.close();require(ready.good(),"cannot acknowledge full-display witness");
        std::filesystem::rename(directory/("witness-"+suffix+".tmp"),directory/("witness-"+suffix+"-ready.txt"));
    }
}
