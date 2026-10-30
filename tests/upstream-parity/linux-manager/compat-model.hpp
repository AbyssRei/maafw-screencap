// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Whole original compat cpp files are separate target sources. Only their host
// authorization calls are the declared script/ownership boundary, not Portal.
#include "MaaControlUnit/WlRootsControlUnitAPI.h"
#include "MaaControlUnit/KWinControlUnitAPI.h"
#include "MaaToolkit/Portal/MaaToolkitPortal.h"
#include <algorithm>
#include <charconv>
#include <vector>
struct MaaToolkitPortalHelper {};
namespace {
std::vector<std::unique_ptr<MaaToolkitPortalHelper>> compat_helpers;
std::vector<std::string> compat_events;
MaaBool compat_open=0;
int compat_fd=-1;
uint32_t compat_node=0;
void compat_helper(MaaToolkitPortalHelper* helper) {
    require(std::any_of(compat_helpers.begin(),compat_helpers.end(),[&](const auto& p){return p.get()==helper;}),"absent/destroyed compat helper");
}
template<class T> T compat_number(const char* input) {
    T value{};const std::string_view text=input;
    const auto parsed=std::from_chars(text.data(),text.data()+text.size(),value);
    require(parsed.ec==std::errc{} && parsed.ptr==text.data()+text.size(),"invalid compatibility numeric argument");return value;
}
std::string compat_unhex(std::string_view text) {
    require(text.size()%2==0,"invalid compatibility path hex");std::string result;
    for(size_t i=0;i<text.size();i+=2) {
        unsigned byte=0;const auto parsed=std::from_chars(text.data()+i,text.data()+i+2,byte,16);
        require(parsed.ec==std::errc{} && parsed.ptr==text.data()+i+2 && byte<=255,"invalid compatibility path byte");
        result.push_back(static_cast<char>(byte));
    }
    return result;
}
}
extern "C" MaaToolkitPortalHelper* MaaToolkitPortalHelperCreate() {
    compat_events.push_back("create");auto owner=std::make_unique<MaaToolkitPortalHelper>();
    auto* result=owner.get();compat_helpers.push_back(std::move(owner));return result;
}
extern "C" MaaBool MaaToolkitPortalHelperOpenStream(MaaToolkitPortalHelper* helper) {
    compat_helper(helper);compat_events.push_back("open");return compat_open;
}
extern "C" int MaaToolkitPortalHelperGetPipeWireFD(MaaToolkitPortalHelper* helper) {
    compat_helper(helper);compat_events.push_back("fd");return compat_fd;
}
extern "C" uint32_t MaaToolkitPortalHelperGetPipeWireNodeID(MaaToolkitPortalHelper* helper) {
    compat_helper(helper);compat_events.push_back("node");return compat_node;
}
extern "C" void MaaToolkitPortalHelperDestroy(MaaToolkitPortalHelper* helper) {
    compat_helper(helper);compat_events.push_back("destroy");
    const auto found=std::find_if(compat_helpers.begin(),compat_helpers.end(),[&](const auto& p){return p.get()==helper;});
    compat_helpers.erase(found);
}
extern "C" MaaBool MaaToolkitPortalHelperGetPersist(MaaToolkitPortalHelper*) {throw std::logic_error("persist outside compat fixture");}
extern "C" void MaaToolkitPortalHelperSetPersist(MaaToolkitPortalHelper*,MaaBool) {throw std::logic_error("persist outside compat fixture");}
extern "C" const char* MaaToolkitPortalHelperGetRestoreToken(MaaToolkitPortalHelper*) {throw std::logic_error("restore token outside compat fixture");}
extern "C" void MaaToolkitPortalHelperSetRestoreToken(MaaToolkitPortalHelper*,const char*) {throw std::logic_error("restore token outside compat fixture");}
int compat_main(int argc,char** argv) {
    require(argc==10,"usage: --compat wlr|kwin path-hex|<null> width height vk open fd node");
    const bool kwin=std::string_view(argv[2])=="kwin";
    require(kwin || std::string_view(argv[2])=="wlr","unknown compatibility method");
    const std::string path=std::string_view(argv[3])=="<null>" ? "" : compat_unhex(argv[3]);
    const char* offered=std::string_view(argv[3])=="<null>" ? nullptr : path.c_str();
    const int width=compat_number<int>(argv[4]),height=compat_number<int>(argv[5]);
    const auto vk=compat_number<unsigned>(argv[6]),open=compat_number<unsigned>(argv[7]);
    require(vk<=255 && open<=255,"legacy MaaBool argument overflow");
    compat_open=static_cast<MaaBool>(open);compat_fd=compat_number<int>(argv[8]);compat_node=compat_number<uint32_t>(argv[9]);
    require(std::string_view(MaaWlRootsControlUnitGetVersion())=="mfs-p1-config-oracle"
        && std::string_view(MaaKWinControlUnitGetVersion())=="mfs-p1-config-oracle","compat version forwarding differs");
    MaaWlRootsControlUnitDestroy(nullptr);MaaKWinControlUnitDestroy(nullptr);
    const auto destroy=kwin ? MaaKWinControlUnitDestroy : MaaWlRootsControlUnitDestroy;
    std::unique_ptr<MAA_CTRL_UNIT_NS::LinuxControlUnitAPI,decltype(destroy)> unit(
        kwin ? MaaKWinControlUnitCreate(offered,width,height,static_cast<MaaBool>(vk))
             : MaaWlRootsControlUnitCreate(offered,static_cast<MaaBool>(vk)),destroy);
    std::cout<<"created="<<static_cast<bool>(unit)<<'\n';
    if(unit) {
        require(!unit->connected() && unit->get_features()==0,"compat construction connected native/input state");
        std::string uuid;require(unit->request_uuid(uuid),"compat original UUID failed");
        std::cout<<"uuid_hex="<<hex(uuid)<<"\ninfo_hex="<<hex(json::value(unit->get_info()).to_string())<<'\n';
        cv::Mat output(1,1,CV_8UC3,cv::Scalar(7,9,11));const auto* pointer=output.data;const int owners=output.u->refcount;
        require(!unit->screencap(output) && output.data==pointer && output.u->refcount==owners,"compat unconnected output changed");
        require(unit->inactive(),"compat unconnected inactive failed");unit.reset();
        require(output.data==pointer && output.u->refcount==owners && output.at<cv::Vec3b>(0,0)==cv::Vec3b(7,9,11),"compat virtual destruction changed caller image");
    }
    std::cout<<"portal_events=";
    for(size_t i=0;i<compat_events.size();++i){if(i){std::cout<<',';}std::cout<<compat_events[i];}
    const auto residual=compat_helpers.size();std::cout<<"\nportal_live="<<residual<<'\n';
    // Failed Open leaks the source helper. Record it BEFORE fixture reclaim;
    // these fake host objects own no fd, authorization or native connection.
    compat_helpers.clear();
    std::cout<<"fixture_reclaimed="<<residual<<"\nno_native_connection=1\n";return 0;
}
