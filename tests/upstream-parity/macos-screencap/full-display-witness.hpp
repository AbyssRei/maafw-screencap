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
        struct BitmapOwner {NSBitmapImageRep* value;~BitmapOwner(){[value release];}} bitmap{
            [[view bitmapImageRepForCachingDisplayInRect:bounds] retain]};
        require(bitmap.value!=nil,"view CPU bitmap allocation unavailable");
        [view cacheDisplayInRect:bounds toBitmapImageRep:bitmap.value];
        require([bitmap.value pixelsWide]==geometry.width && [bitmap.value pixelsHigh]==geometry.height,
            "view CPU bitmap does not have the required 1:1 dimensions");
        const auto wide=static_cast<size_t>(geometry.width),high=static_cast<size_t>(geometry.height);
        require(wide<=std::numeric_limits<size_t>::max()/4 && high<=std::numeric_limits<size_t>::max()/(wide*4),
            "view witness byte span overflow");
        const auto span=wide*4*high;
        require(span<=static_cast<size_t>(std::numeric_limits<std::streamsize>::max()),"view witness write narrowing");
        std::vector<unsigned char> bytes(span);
        const auto component=[](CGFloat value) {
            require(std::isfinite(value) && value>=0 && value<=1,"view sRGB component out of range");
            const auto rounded=std::lround(value*255.0);
            require(rounded>=0 && rounded<=255,"view component narrowing");
            return static_cast<unsigned char>(rounded);
        };
        for(int y=0;y<geometry.height;++y) {
            // Bound temporary NSColor owners even for large full-view images.
            @autoreleasepool {
                for(int x=0;x<geometry.width;++x) {
                    // colorAtX:y: uses the bitmap's documented top-left origin.
                    // Semantic sRGB components avoid assuming native byte order,
                    // premultiplication, samples-per-pixel or source row pitch.
                    NSColor* colour=[[bitmap.value colorAtX:x y:y] colorUsingColorSpace:[NSColorSpace sRGBColorSpace]];
                    require(colour!=nil,"view CPU pixel cannot be represented in sRGB");
                    CGFloat red=0,green=0,blue=0,alpha=0;
                    [colour getRed:&red green:&green blue:&blue alpha:&alpha];
                    const auto offset=(static_cast<size_t>(y)*wide+static_cast<size_t>(x))*4;
                    bytes[offset]=component(blue);bytes[offset+1]=component(green);
                    bytes[offset+2]=component(red);bytes[offset+3]=component(alpha);
                }
            }
        }
        const auto suffix=std::to_string(slot);
        std::ofstream out(directory/("view-"+suffix+".bgra"),std::ios::binary);
        out.write(reinterpret_cast<const char*>(bytes.data()),static_cast<std::streamsize>(span));
        out.close();require(out.good(),"cannot save view CPU witness");
        full_witness_json(directory/("view-"+suffix+".json"),@{
            @"schema":@1,@"slot":@(slot),@"pid":@(getpid()),@"main_thread":@1,
            @"window_id":@(geometry.window),@"width":@(geometry.width),@"height":@(geometry.height),
            @"source_bits_per_sample":@([bitmap.value bitsPerSample]),
            @"source_bits_per_pixel":@([bitmap.value bitsPerPixel]),
            @"source_samples_per_pixel":@([bitmap.value samplesPerPixel]),
            @"source_bytes_per_row":@([bitmap.value bytesPerRow]),
            @"source_bitmap_format":@(static_cast<uint64_t>([bitmap.value bitmapFormat])),
            @"source_has_alpha":@([bitmap.value hasAlpha]!=NO),@"source_planar":@([bitmap.value isPlanar]!=NO),
            @"canonical_format":@"BGRA8_sRGB",@"canonical_stride":@(wide*4),
            @"top_left_origin":@1,@"origin":@"NSView_cacheDisplay_semantic_NSColor"});

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
            require(visible && CFGetTypeID(visible)==CFBooleanGetTypeID(),"window census visibility missing");
            NSMutableDictionary* row=[NSMutableDictionary dictionaryWithDictionary:@{
                @"rank":@(index),@"id":@(number),@"pid":@(pid),@"layer":@(layer),
                @"onscreen":@(CFBooleanGetValue(static_cast<CFBooleanRef>(visible))!=0),
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
                    CFBooleanGetValue(static_cast<CFBooleanRef>(visible)),"window census substituted fixture owner/bounds/level");
                require(!found,"window census repeated owned target");found=true;
            }
        }
        require(found,"owned full-display window missing from census");
        full_witness_json(directory/("census-"+suffix+".json"),@{
            @"schema":@1,@"slot":@(slot),@"pid":@(getpid()),@"main_thread":@1,
            @"window_id":@(geometry.window),@"display_id":@(geometry.id),@"windows":observed});
        std::ofstream ready(directory/("witness-"+suffix+".tmp"),std::ios::binary);
        ready<<"slot="<<slot<<"\nmain_thread=1\n";ready.close();require(ready.good(),"cannot acknowledge full-display witness");
        std::filesystem::rename(directory/("witness-"+suffix+".tmp"),directory/("witness-"+suffix+"-ready.txt"));
    }
}
