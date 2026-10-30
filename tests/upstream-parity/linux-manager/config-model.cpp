// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Exact legacy create/destroy, Config.h and queried manager bodies, using real
// public C++ handles/vtables, meojson and Mat. No backend/input connection runs.
#include "MaaControlUnit/LinuxControlUnitAPI.h"
#include "Manager/LinuxControlUnitMgr.h"
#include "MaaUtils/Logger.h"
#include <fstream>
#include <iostream>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>
#include <string_view>

MAA_CTRL_UNIT_NS_BEGIN
#include "linux-config-manager.inc"
// Full original class declarations remain authoritative. These unrelated
// virtual input/native-connect entries explicitly reject any accidental use;
// they are not migrated input behavior or a substitute capture implementation.
bool LinuxControlUnitMgr::connect() {throw std::logic_error("native connect excluded from config oracle");}
bool LinuxControlUnitMgr::start_app(const std::string&) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::stop_app(const std::string&) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::click(int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::swipe(int,int,int,int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::touch_down(int,int,int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::touch_move(int,int,int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::touch_up(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::click_key(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::input_text(const std::string&) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::key_down(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::key_up(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::scroll(int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::relative_move(int,int) {throw std::logic_error("input excluded");}
MAA_CTRL_UNIT_NS_END
#include "linux-config-api.inc"

namespace {
void require(bool value,const char* message) {if(!value) {throw std::runtime_error(message);}}
std::string hex(std::string_view bytes) {
    require(bytes.size()<=std::numeric_limits<size_t>::max()/2,"hex size overflow");
    std::string result;result.reserve(bytes.size()*2);
    constexpr char digits[]="0123456789abcdef";
    for(unsigned char byte:bytes) {result.push_back(digits[byte>>4]);result.push_back(digits[byte&15]);}
    return result;
}
}
#include "compat-model.hpp"
int main(int argc,char** argv) {
    try {
        static_assert(sizeof(MaaLinuxScreencapMethod)==8 && sizeof(MaaLinuxInputMethod)==8);
        static_assert(sizeof(int)==4 && sizeof(uint32_t)==4);
        static_assert(json::_packed_bytes::packed_bytes_trait_max::step==16,
            "Recharacterize the explicit source parser before changing its packed-byte trait");
        require(std::string_view(MaaLinuxControlUnitGetVersion())=="mfs-p1-config-oracle","original API version forwarding changed");
        if(argc>1 && std::string_view(argv[1])=="--compat") { return compat_main(argc,argv); }
        require(argc==2 || argc==3,"usage: config-model --null | --file <raw JSON file>");
        std::string input;
        const char* offered=nullptr;
        if(argc==3 && std::string_view(argv[1])=="--file") {
            std::ifstream file(argv[2],std::ios::binary);
            require(file.good(),"cannot open raw config case");
            input.assign(std::istreambuf_iterator<char>(file),std::istreambuf_iterator<char>());
            require(!file.bad(),"cannot read raw config case");
            offered=input.c_str(); // preserve the original C-string boundary
        }else {require(argc==2 && std::string_view(argv[1])=="--null","invalid config case arguments");}
        MaaLinuxControlUnitDestroy(nullptr);
        std::unique_ptr<MAA_CTRL_UNIT_NS::LinuxControlUnitAPI,decltype(&MaaLinuxControlUnitDestroy)> unit(
            MaaLinuxControlUnitCreate(offered),MaaLinuxControlUnitDestroy);
        std::cout<<"created="<<static_cast<bool>(unit)<<'\n';
        if(unit) {
            require(!unit->connected(),"configuration API unexpectedly connected its manager");
            std::string uuid="unchanged";
            require(unit->request_uuid(uuid),"original config UUID failed");
            std::cout<<"connected=0\nuuid_hex="<<hex(uuid)<<"\nfeatures="<<unit->get_features()<<'\n';
            const auto info=unit->get_info();
            std::cout<<"info_hex="<<hex(json::value(info).to_string())<<'\n';
            cv::Mat output(1,1,CV_8UC3,cv::Scalar(7,9,11));
            const auto* pointer=output.data;
            const int owners=output.u->refcount;
            require(!unit->screencap(output) && output.data==pointer && output.u->refcount==owners,
                "unconnected original API changed image ownership");
            require(unit->inactive(),"unconnected original API inactive failed");
            unit.reset();
            require(output.data==pointer && output.u->refcount==owners && output.at<cv::Vec3b>(0,0)==cv::Vec3b(7,9,11),
                "original API destruction changed caller pixels");
            std::cout<<"no_native_connection=1\nfailed_output_preserved=1\ndestroyed=1\n";
        }
        return 0;
    }catch(const std::exception& error) {
        // The original API does not add a catch. Observe exceptions at the C++
        // test caller, without claiming a safe cross-language exception boundary.
        std::cout<<"caller_caught_exception_hex="<<hex(error.what())<<'\n';return 0;
    }
}
