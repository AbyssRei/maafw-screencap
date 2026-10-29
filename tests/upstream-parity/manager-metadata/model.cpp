// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Exact original get_info bodies with copied member declarations and real
// MaaDef/meojson. Direct field query model only: no native manager/virtual ABI,
// constructor/background-input object, source connection or desktop capture.
#if defined(_WIN32) && !defined(NOMINMAX)
#define NOMINMAX
#endif
#include "MaaFramework/MaaDef.h"
#include <meojson/json.hpp>
#include <charconv>
#include <cstdint>
#include <fstream>
#include <iostream>
#include <limits>
#include <sstream>
#include <stdexcept>
#include <string>
#if defined(_WIN32)
#include <windows.h>
#else
using HWND=void*; // Explicit portable pointer projection; never dereferenced.
#endif
namespace Win {
struct Win32ControlUnitMgr {
    json::object get_info() const;
#include "win32-info-members.inc"
};
#include "win32-info-source.inc"
}
namespace Mac {
struct MacOSControlUnitMgr {
    json::object get_info() const;
#include "macos-info-members.inc"
};
#include "macos-info-source.inc"
}
namespace {
void require(bool value,const char* message) {if(!value) {throw std::runtime_error(message);}}
uint64_t number(const std::string& text) {
    uint64_t value=0;const auto result=std::from_chars(text.data(),text.data()+text.size(),value);
    require(result.ec==std::errc{} && result.ptr==text.data()+text.size(),"invalid metadata model number");return value;
}
}
int main(int argc,char** argv) {
    try {
        static_assert(sizeof(HWND)==8 && sizeof(uintptr_t)==8);
        static_assert(sizeof(MaaWin32ScreencapMethod)==8 && sizeof(MaaWin32InputMethod)==8);
        static_assert(sizeof(MaaMacOSScreencapMethod)==8 && sizeof(MaaMacOSInputMethod)==8);
        require(argc==2,"usage: model <explicit case table>");
        std::ifstream input(argv[1],std::ios::binary);require(input.good(),"cannot open metadata case table");
        std::string line;size_t row=0;
        while(std::getline(input,line)) {
            std::istringstream fields(line);std::string kind,id_text,method_text,extra;
            require(static_cast<bool>(fields>>kind>>id_text>>method_text) && !(fields>>extra),"invalid metadata case row");
            const uint64_t id=number(id_text),method=number(method_text);
            json::object info;
            if(kind=="win32") {
                Win::Win32ControlUnitMgr manager;
                manager.hwnd_=reinterpret_cast<HWND>(static_cast<uintptr_t>(id));
                manager.screencap_method_=method;
                require(manager.mouse_method_==0 && manager.keyboard_method_==0,"metadata model input must remain None");
                info=manager.get_info();
            }else {
                require(kind=="macos" && id<=std::numeric_limits<uint32_t>::max(),"invalid macOS metadata target");
                Mac::MacOSControlUnitMgr manager;
                manager.window_id_=static_cast<uint32_t>(id);manager.screencap_method_=method;
                require(manager.input_method_==0,"metadata model input must remain None");
                info=manager.get_info();
            }
            std::cout<<row<<'\t'<<json::value(info).to_string()<<'\n';
            require(row<std::numeric_limits<size_t>::max(),"metadata row count overflow");++row;
        }
        require(input.eof() && !input.bad() && row>0 && std::cout.good(),"metadata table/output I/O failed");
        return 0;
    }catch(const std::exception& error) {std::cerr<<error.what()<<'\n';return 1;}
}
