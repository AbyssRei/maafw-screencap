// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// In-memory input-fixture control: no desktop capture or shown/activated window.
#include <windows.h>
#include <array>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>

namespace {
constexpr size_t WIDTH=96,HEIGHT=64,BYTES=WIDTH*HEIGHT*4;
struct Observation {
    const uint8_t* pixels=nullptr;
    std::filesystem::path root;
    std::ofstream rows;
    unsigned draw=0,fill=0,total=0,valid=0,unmarked=0,other=0,errors=0;
};
thread_local Observation* observation=nullptr;
std::array<uint8_t,3> pixel(size_t x,size_t y,int phase) {
    if(phase>=0 && x<16 && y<16)
        return phase ? std::array<uint8_t,3>{0x1a,0x2b,0x3c} : std::array<uint8_t,3>{0x4e,0x5f,0x60};
    return x<32 ? std::array<uint8_t,3>{0x1f,0x53,0xa7} : y<32 ? std::array<uint8_t,3>{0xb9,0x4d,0x27} : std::array<uint8_t,3>{0x3b,0xe2,0x81};
}
int phase(const uint8_t* pixels) {
    const auto first=std::array<uint8_t,3>{pixels[0],pixels[1],pixels[2]};
    int result=first==pixel(0,0,0) ? 0 : first==pixel(0,0,1) ? 1 : first==pixel(0,0,-1) ? -1 : -2;
    if(result==-2) return result;
    for(size_t y=0;y<HEIGHT;++y) for(size_t x=0;x<WIDTH;++x)
        if(std::memcmp(pixels+(y*WIDTH+x)*4,pixel(x,y,result).data(),3)!=0) return -2;
    return result;
}
int WINAPI observed_fill(HDC dc,const RECT* rectangle,HBRUSH brush) noexcept {
    const int result=::FillRect(dc,rectangle,brush);
    if(observation) {
        auto& o=*observation;
        try {
            // Exact synchronous GDI operation boundary on our private DIB.
            // This extra flush is test instrumentation, not capture timing.
            if(!GdiFlush()) throw std::runtime_error("control GdiFlush failed");
            const int p=phase(o.pixels);
            if(p>=0) ++o.valid;else if(p==-1) ++o.unmarked;else ++o.other;
            const auto name="draw-"+std::to_string(o.draw)+"-fill-"+std::to_string(o.fill)+".bgra";
            std::ofstream bytes(o.root/name,std::ios::binary);
            bytes.write(reinterpret_cast<const char*>(o.pixels),BYTES);bytes.close();
            if(!bytes.good()) throw std::runtime_error("control pixel write failed");
            o.rows<<o.draw<<'\t'<<o.fill<<'\t'<<p<<'\n';o.rows.flush();
            if(!o.rows.good()) throw std::runtime_error("control row write failed");
            ++o.fill;++o.total;
        } catch(...) {++o.errors;}
    }
    return result;
}
struct Surface {
    HWND window=nullptr;HDC dc=nullptr;HBITMAP bitmap=nullptr;uint8_t* pixels=nullptr;
    unsigned errors=0;
    void open() {
        window=CreateWindowExW(0,L"STATIC",L"mfs hidden marker control",WS_POPUP,0,0,WIDTH,HEIGHT,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
        if(!window || IsWindowVisible(window)) throw std::runtime_error("hidden control window failed");
        dc=CreateCompatibleDC(nullptr);
        BITMAPINFO info{};info.bmiHeader.biSize=sizeof(BITMAPINFOHEADER);
        info.bmiHeader.biWidth=WIDTH;info.bmiHeader.biHeight=-static_cast<LONG>(HEIGHT);
        info.bmiHeader.biPlanes=1;info.bmiHeader.biBitCount=32;info.bmiHeader.biCompression=BI_RGB;
        void* output=nullptr;
        bitmap=CreateDIBSection(dc,&info,DIB_RGB_COLORS,&output,nullptr,0);pixels=static_cast<uint8_t*>(output);
        if(!dc || !bitmap || !pixels) throw std::runtime_error("control DIB failed");
        const auto previous=SelectObject(dc,bitmap);
        if(!previous || previous==HGDI_ERROR) throw std::runtime_error("control bitmap select failed");
    }
    void seed() {
        if(!GdiFlush()) throw std::runtime_error("seed GdiFlush failed");
        for(size_t y=0;y<HEIGHT;++y) for(size_t x=0;x<WIDTH;++x) {
            auto* target=pixels+(y*WIDTH+x)*4;const auto color=pixel(x,y,0);
            std::memcpy(target,color.data(),3);target[3]=0;
        }
    }
    void close() noexcept {
        // Deleting our DC ends its bitmap selection; the DIB remains separately
        // owned and is then deleted once. No borrowed pixels survive this call.
        if(dc) {if(!DeleteDC(dc))++errors;dc=nullptr;}
        if(bitmap) {if(!DeleteObject(bitmap))++errors;bitmap=nullptr;pixels=nullptr;}
        if(window) {if(!DestroyWindow(window))++errors;window=nullptr;}
    }
    ~Surface(){close();}
};
}

// Include the actual renderer in this isolated TU. Only FillRect is observed;
// no product/backend API or shared fixture declarations are changed.
#define FillRect observed_fill
#ifndef MFS_MARKER_RENDER_CPP
#define MFS_MARKER_RENDER_CPP "print-parity.cpp"
#endif
#include MFS_MARKER_RENDER_CPP
#undef FillRect

int main(int argc,char** argv) {
    try {
        if(argc!=2) throw std::runtime_error("usage: marker-control EMPTY_ABSOLUTE_ROOT");
        Observation o;o.root=std::filesystem::path(argv[1]);
        if(!o.root.is_absolute() || !std::filesystem::is_directory(o.root) || !std::filesystem::is_empty(o.root)) throw std::runtime_error("control root must be new and empty");
        // Warm the actual clipping/brush path twice before resource accounting;
        // require a stable count rather than treating lazy GDI state as a leak.
        DWORD warm_count=0;
        for(unsigned pass=0;pass<2;++pass) {
            Surface warm;warm.open();warm.seed();
            for(unsigned clip=0;clip<2;++clip) {
                if(SelectClipRgn(warm.dc,nullptr)==ERROR || (clip && IntersectClipRect(warm.dc,0,0,16,16)==ERROR))
                    throw std::runtime_error("warm clip failed");
                Pattern pattern;pattern.inside_marker=true;
                draw(warm.window,warm.dc,pattern);
                if(pattern.failed)throw std::runtime_error("warm renderer failed");
            }
            warm.close();if(warm.errors || !GdiFlush())throw std::runtime_error("warm cleanup failed");
            const auto count=GetGuiResources(GetCurrentProcess(),GR_GDIOBJECTS);
            if(pass && count!=warm_count)throw std::runtime_error("warm resource count changed");
            warm_count=count;
        }
        const auto baseline=GetGuiResources(GetCurrentProcess(),GR_GDIOBJECTS);
        Surface surface;surface.open();o.pixels=surface.pixels;
        o.rows.open(o.root/"operations.tsv");o.rows<<"draw\tfill\tphase\n";
        for(unsigned clip=0;clip<2;++clip) for(unsigned repaint=0;repaint<2;++repaint) {
            surface.seed();
            if(SelectClipRgn(surface.dc,nullptr)==ERROR)throw std::runtime_error("control clip clear failed");
            if(clip && IntersectClipRect(surface.dc,0,0,16,16)==ERROR)throw std::runtime_error("control marker clip failed");
            RECT before{},after{};const int before_kind=GetClipBox(surface.dc,&before);
            Pattern pattern;pattern.inside_marker=true;pattern.marker_alternative=repaint==0;
            o.draw=clip*2+repaint;o.fill=0;observation=&o;
            draw(surface.window,surface.dc,pattern);
            observation=nullptr;
            const int after_kind=GetClipBox(surface.dc,&after);
            if(before_kind==ERROR || before_kind!=after_kind || std::memcmp(&before,&after,sizeof(RECT)) || pattern.failed || o.fill!=4 || phase(surface.pixels)!=static_cast<int>(repaint==0) || GetCurrentObject(surface.dc,OBJ_BITMAP)!=surface.bitmap)
                throw std::runtime_error("renderer output/state differs");
        }
        o.rows.close();if(!o.rows.good())throw std::runtime_error("control table incomplete");
        surface.close();if(!GdiFlush())throw std::runtime_error("final GdiFlush failed");
        const auto final=GetGuiResources(GetCurrentProcess(),GR_GDIOBJECTS);
        std::ofstream summary(o.root/"summary.tsv");
        summary<<"draws\tfills\tvalid\tunmarked\tother\terrors\tgdi_before\tgdi_after\tcleanup\n4\t"<<o.total<<'\t'<<o.valid<<'\t'<<o.unmarked<<'\t'<<o.other<<'\t'<<o.errors<<'\t'<<baseline<<'\t'<<final<<'\t'<<surface.errors<<'\n';summary.close();
        const bool passed=o.total==16 && o.valid==16 && !o.unmarked && !o.other && !o.errors && !surface.errors && baseline==final && summary.good();
        std::cout<<"marker input control draws=4 fills="<<o.total<<" valid="<<o.valid<<" unmarked="<<o.unmarked<<" other="<<o.other<<" errors="<<o.errors<<" gdi="<<baseline<<','<<final<<" cleanup="<<surface.errors<<" hidden=1 native_capture=0 "<<(passed?"passed":"failed")<<'\n';
        return passed?0:1;
    } catch(const std::exception& e) {observation=nullptr;std::cerr<<"marker control failure="<<e.what()<<'\n';return 1;}
}
