// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Explicit query facade, not the complete input/framework manager or SDK.
#include "bridge.h"
#if defined(__APPLE__)
#include "Screencap/ScreenCaptureKitScreencap.h"
#else
#include "Base/UnitBase.h"
#endif
// Screenshot source's private Conf is confined to its declarations above.
// Real public declarations below retain the original MaaNS::CtrlUnitNs ABI.
#undef MAA_CTRL_UNIT_NS
#undef MAA_CTRL_UNIT_NS_BEGIN
#undef MAA_CTRL_UNIT_NS_END
#include "MaaFramework/MaaPort.h"
#undef MAA_CONTROL_UNIT_API
#define MAA_CONTROL_UNIT_API
#include "MaaControlUnit/MacOSControlUnitAPI.h"
#include <cstring>
#include <limits>
#include <optional>
#include <stdexcept>

namespace Source {
using ScreencapBase=MfsOriginalMac::ScreencapBase;
#if defined(__APPLE__)
using ScreenCaptureKitScreencap=MfsOriginalMac::ScreenCaptureKitScreencap;
#else
struct ScreenCaptureKitScreencap : ScreencapBase {
    explicit ScreenCaptureKitScreencap(uint32_t) { throw std::logic_error("portable unavailable branch constructed SCK"); }
    std::optional<cv::Mat> screencap() override { throw std::logic_error("capture excluded from query consumer"); }
};
#endif
inline uint64_t input_attempts=0;
inline void mfs_mac_input_attempt() { ++input_attempts; }
#define MFS_MAC_QUERY_CONSUMER
#include "manager-shim.h"
#undef MFS_MAC_QUERY_CONSUMER
#define LogWarn mfs_mac_null_log
#define LogError mfs_mac_null_log
#if !defined(__APPLE__)
// Explicit model intervention. Native Apple compilation uses the real builtin.
#define __builtin_available(...) false
#endif
#include "macos-manager-source.inc"
#if !defined(__APPLE__)
#undef __builtin_available
#endif
#undef LogError
#undef LogWarn
// Same source manager and state, not a separate metadata field-only oracle.
// Declaration is supplied by the shared shim under this scoped definition.
#include "macos-info-source.inc"
}
namespace {
thread_local const MfsMqHost* constructing_host=nullptr;
MfsMqValue call(const MfsMqHost& host,uint32_t operation) {
    MfsMqValue value={};
    if(!host.context || !host.call || host.call(host.context,operation,&value)) {
        throw std::runtime_error("Rust query callback failed");
    }
    if(value.length>sizeof(value.bytes)) { throw std::logic_error("query span overflow"); }
    return value;
}
std::string text(const MfsMqValue& value) {
    return std::string(reinterpret_cast<const char*>(value.bytes),value.length);
}
}
MAA_CTRL_UNIT_NS_BEGIN
class MacOSControlUnitMgr final : public MacOSControlUnitAPI {
    std::optional<Source::MacOSControlUnitMgr> source_;
    MfsMqHost host_={};
public:
    MacOSControlUnitMgr(uint32_t id,MaaMacOSScreencapMethod method,MaaMacOSInputMethod input) {
        if(input!=MaaMacOSInputMethod_None) { throw std::logic_error("Input=None required"); }
        if(constructing_host) { host_=*constructing_host; }
        else { source_.emplace(id,method,input); }
    }
    bool connect() override { return source_ ? source_->connect() : call(host_,0).scalar!=0; }
    bool connected() const override { return source_ ? source_->connected() : call(host_,1).scalar!=0; }
    bool request_uuid(std::string& uuid) override {
        if(source_) { return source_->request_uuid(uuid); }
        const auto value=call(host_,2);uuid=text(value);return value.scalar!=0;
    }
    MaaControllerFeature get_features() const override { return source_ ? source_->get_features() : call(host_,3).scalar; }
    json::object get_info() const override {
        if(source_) { return source_->get_info(); }
        const auto value=call(host_,4);
        const auto parsed=json::parse(text(value));
        if(!parsed || !parsed->is_object()) { throw std::logic_error("invalid Rust metadata"); }
        return parsed->as_object(); // Explicit extra native parse/representation.
    }
    bool inactive() override { return source_ ? source_->inactive() : call(host_,5).scalar!=0; }
    // No invocation of excluded action/capture slots is accepted by this test.
    bool start_app(const std::string&) override { throw std::logic_error("excluded start_app"); }
    bool stop_app(const std::string&) override { throw std::logic_error("excluded stop_app"); }
    bool screencap(cv::Mat&) override { throw std::logic_error("excluded capture"); }
    bool click(int,int) override { throw std::logic_error("excluded click"); }
    bool swipe(int,int,int,int,int) override { throw std::logic_error("excluded swipe"); }
    bool touch_down(int,int,int,int) override { throw std::logic_error("excluded touch_down"); }
    bool touch_move(int,int,int,int) override { throw std::logic_error("excluded touch_move"); }
    bool touch_up(int) override { throw std::logic_error("excluded touch_up"); }
    bool click_key(int) override { throw std::logic_error("excluded click_key"); }
    bool input_text(const std::string&) override { throw std::logic_error("excluded input_text"); }
    bool key_down(int) override { throw std::logic_error("excluded key_down"); }
    bool key_up(int) override { throw std::logic_error("excluded key_up"); }
    bool scroll(int,int) override { throw std::logic_error("excluded scroll"); }
    bool relative_move(int,int) override { throw std::logic_error("excluded relative_move"); }
};
MAA_CTRL_UNIT_NS_END
#define LogFunc Source::mfs_mac_null_log
#define VAR(x) x
#define VAR_VOIDP(x) x
extern "C" {
#include "macos-factory-source.inc"
}
#undef VAR_VOIDP
#undef VAR
#undef LogFunc

size_t mfs_mq_layout(uint32_t index) noexcept {
    constexpr size_t values[]={1,
        sizeof(MfsMqValue),alignof(MfsMqValue),offsetof(MfsMqValue,scalar),offsetof(MfsMqValue,length),offsetof(MfsMqValue,bytes),
        sizeof(MfsMqHost),alignof(MfsMqHost),offsetof(MfsMqHost,context),offsetof(MfsMqHost,call),
        sizeof(MfsMqSnapshot),alignof(MfsMqSnapshot),offsetof(MfsMqSnapshot,features),offsetof(MfsMqSnapshot,connected),
        offsetof(MfsMqSnapshot,uuid_success),offsetof(MfsMqSnapshot,uuid_length),offsetof(MfsMqSnapshot,info_length),
        offsetof(MfsMqSnapshot,uuid),offsetof(MfsMqSnapshot,info),
        sizeof(MaaMacOSScreencapMethod),sizeof(MaaMacOSInputMethod),sizeof(MaaControllerFeature),
        MaaMacOSScreencapMethod_None,MaaMacOSScreencapMethod_ScreenCaptureKit,MaaMacOSInputMethod_None};
    return index<sizeof(values)/sizeof(values[0]) ? values[index] : std::numeric_limits<size_t>::max();
}
uint32_t mfs_mq_environment() noexcept {
#if defined(__APPLE__)
    return __builtin_available(macOS 14.0, *) ? 1 : 2;
#else
    return 0;
#endif
}
int32_t mfs_mq_create(uint32_t id,uint64_t method,const MfsMqHost* host,void** output) noexcept {
    if(!output) { return 1; }*output=nullptr;
    if(constructing_host || (host && (!host->context || !host->call))) { return 1; }
    constructing_host=host;
    struct Reset { ~Reset() { constructing_host=nullptr; } } reset;
    try { *output=MaaMacOSControlUnitCreate(id,method,MaaMacOSInputMethod_None);return *output ? 0 : 2; }
    catch(...) { return 2; }
}
int32_t mfs_mq_connect(void* unit,uint32_t* output) noexcept {
    if(!output) { return 1; }*output=0;if(!unit) { return 1; }
    try { *output=static_cast<MaaMacOSControlUnitHandle>(unit)->connect();return 0; }catch(...) { return 2; }
}
int32_t mfs_mq_inactive(void* unit,uint32_t* output) noexcept {
    if(!output) { return 1; }*output=0;if(!unit) { return 1; }
    try { *output=static_cast<MaaMacOSControlUnitHandle>(unit)->inactive();return 0; }catch(...) { return 2; }
}
int32_t mfs_mq_query(void* unit,MfsMqSnapshot* output) noexcept {
    if(!output) { return 1; }*output={};if(!unit) { return 1; }
    try {
        const auto handle=static_cast<MaaMacOSControlUnitHandle>(unit);
        MfsMqSnapshot value={};std::string uuid="caller sentinel";
        value.connected=handle->connected();value.uuid_success=handle->request_uuid(uuid);
        value.features=handle->get_features();
        if(Source::input_attempts) { throw std::logic_error("unexpected input consumption"); }
        const auto info=json::value(handle->get_info()).to_string();
        if(uuid.size()>sizeof(value.uuid) || info.size()>sizeof(value.info)) { throw std::logic_error("snapshot capacity"); }
        value.uuid_length=uuid.size();value.info_length=info.size();
        std::memcpy(value.uuid,uuid.data(),uuid.size());std::memcpy(value.info,info.data(),info.size());
        *output=value;return 0;
    }catch(...) { return 2; }
}
int32_t mfs_mq_destroy(void* unit) noexcept {
    try { MaaMacOSControlUnitDestroy(static_cast<MaaMacOSControlUnitHandle>(unit));return 0; }catch(...) { return 2; }
}
