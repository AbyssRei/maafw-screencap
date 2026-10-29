// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Real Boost DLL boundary, complete original loader and outer capture bodies.
#include "bridge.h"
#include "MaaControlUnit/Win32ControlUnitAPI.h"
#include "original-LibraryHolder.h"
#include <windows.h>
#include <iostream>
#include <memory>
#include <filesystem>
namespace MaaNS {
std::filesystem::path directory;
inline std::filesystem::path library_dir() { return directory; }
}
namespace Source {
using namespace MaaNS;
using namespace MAA_CTRL_UNIT_NS;
#include "delegate-loader.inc"
struct GamepadControlUnitMgr {
    bool init_win32_unit();bool screencap(cv::Mat&);bool inactive();
    HWND hwnd_=nullptr;
    MaaWin32ScreencapMethod screencap_method_=MaaWin32ScreencapMethod_GDI;
    std::unique_ptr<Win32ControlUnitLoader> win32_loader_;
    std::shared_ptr<ControlUnitAPI> win32_unit_;
};
#include "delegate-source.inc"
}
int wmain(int argc,wchar_t** argv) {
    try {
        if(argc!=4) { throw std::logic_error("expected directory hwnd output"); }
        MaaNS::directory=argv[1];
        cv::Mat image;
        {
            Source::GamepadControlUnitMgr manager;
            manager.hwnd_=reinterpret_cast<HWND>(std::stoull(argv[2]));
            if(!manager.init_win32_unit() || !manager.screencap(image) || !manager.inactive()) { throw std::runtime_error("original native delegate call failed"); }
            // Copy to executable-owned CPU bytes WHILE loader/code remains live.
            // Exact outer capture above still executes original body directly.
            image=image.clone();
        }
        for(const auto* name:{L"MaaWin32ControlUnit.dll",L"libMaaWin32ControlUnit.dll",L"MaaWin32ControlUnit."}) {
            if(GetModuleHandleW((MaaNS::directory/name).c_str())) { throw std::runtime_error("source module still loaded"); }
        }
        if(image.type()!=CV_8UC3 || image.empty()) { throw std::runtime_error("invalid source pixels"); }
        FILE* file=nullptr;
        if(_wfopen_s(&file,argv[3],L"wb") || !file) { throw std::runtime_error("open output"); }
        bool failed=false;
        for(int y=0;y<image.rows;++y) { if(fwrite(image.ptr(y),3,image.cols,file)!=static_cast<size_t>(image.cols)) { failed=true; } }
        if(fclose(file) || failed) { throw std::runtime_error("write source output"); }
        std::cout<<"source_native_delegate=1\nwidth="<<image.cols<<"\nheight="<<image.rows<<"\nmodule_unloaded=1\n";
        return 0;
    } catch(const std::exception& error) { std::cerr<<error.what()<<'\n';return 1; }
}
