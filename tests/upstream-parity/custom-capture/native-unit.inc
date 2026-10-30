// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included in bridge.cpp to retain its concrete Mat/string/JSON owner types.
// The two complete original .cpp files are separate unmodified translation units.
#include "MaaControlUnit/CustomControlUnitAPI.h"
namespace {
std::atomic<uint64_t> unit_counts[7]{}; // source/Rust create/destroy/dtor, scoped UUID/Mat publications
MaaBool unit_bool_fault(void*) {throw std::runtime_error("controlled original boolean callback exception");}
struct RustUnitError : std::exception {
    int32_t code;
    explicit RustUnitError(int32_t value):code(value){}
};
struct UnitResult {
    void* object=nullptr;
    void* destination=nullptr;
    uint64_t value=0;
    uint32_t operation;
    explicit UnitResult(uint32_t op):operation(op){}
    UnitResult(const UnitResult&)=delete;
    UnitResult& operator=(const UnitResult&)=delete;
    ~UnitResult(){
        if(operation==4)delete static_cast<UuidOutput*>(object);
        else if(operation==5)delete static_cast<Image*>(object);
        else if(operation==6)delete static_cast<Info*>(object);
    }
};
class RustUnit final : public MAA_CTRL_UNIT_NS::CustomControlUnitAPI {
    MfsCustomUnitDispatch dispatch_;
    MfsCustomUnitDrop drop_;
    void* argument_;
    void call(UnitResult& result) const {
        const auto status=dispatch_(argument_,result.operation,result.destination,&result.object,&result.value);
        if(status!=0)throw RustUnitError(status);
        if(result.operation<6 && result.object)throw std::runtime_error("unexpected native owner");
        if((result.operation==4 || result.operation==5) && result.value>1)throw std::runtime_error("invalid result byte");
        if(result.operation==6 && result.value && !result.object)throw std::runtime_error("successful native value missing");
    }
    uint64_t scalar(uint32_t op) const {UnitResult result(op);call(result);return result.value;}
public:
    RustUnit(MfsCustomUnitDispatch dispatch,MfsCustomUnitDrop drop,void* argument):dispatch_(dispatch),drop_(drop),argument_(argument){}
    ~RustUnit() override {++unit_counts[4];drop_(argument_);}
    bool connect() override {return scalar(0)!=0;}
    bool connected() const override {return scalar(1)!=0;}
    bool inactive() override {return scalar(2)!=0;}
    MaaControllerFeature get_features() const override {return scalar(3);}
    bool request_uuid(std::string& uuid) override {
        UnitResult result(4);result.destination=&uuid;call(result);
        return result.value!=0;
    }
    bool screencap(cv::Mat& image) override {
        UnitResult result(5);result.destination=&image;call(result);
        return result.value!=0;
    }
    json::object get_info() const override {
        UnitResult result(6);call(result);
        if(result.value!=1)throw std::runtime_error("metadata owner missing");
        return std::move(static_cast<Info*>(result.object)->value);
    }
    // Vtable completeness only; input/app/shell operations are outside capture.
    // These slots are never invoked by this acceptance consumer.
    bool start_app(const std::string&) override {return false;}
    bool stop_app(const std::string&) override {return false;}
    bool click(int,int) override {return false;}
    bool swipe(int,int,int,int,int) override {return false;}
    bool touch_down(int,int,int,int) override {return false;}
    bool touch_move(int,int,int,int) override {return false;}
    bool touch_up(int) override {return false;}
    bool click_key(int) override {return false;}
    bool input_text(const std::string&) override {return false;}
    bool key_down(int) override {return false;}
    bool key_up(int) override {return false;}
    bool scroll(int,int) override {return false;}
    bool relative_move(int,int) override {return false;}
    bool shell(const std::string&,std::string&,std::chrono::milliseconds) override {return false;}
};
}
size_t mfs_custom_unit_abi(uint32_t index) noexcept {
    const size_t values[]={sizeof(MfsCustomUnitDispatch),sizeof(MfsCustomUnitDrop),
        sizeof(decltype(&MaaCustomControlUnitCreate)),sizeof(decltype(&MaaCustomControlUnitDestroy)),
        sizeof(MaaCustomControlUnitHandle),sizeof(MaaControllerFeature)};
    return index<std::size(values)?values[index]:SIZE_MAX;
}
int32_t mfs_custom_unit_source_create(void* context,void** output) noexcept {
    if(!output)return 1;*output=nullptr;if(!context)return 1;
    return guarded([&]{auto& c=*static_cast<Context*>(context);
        *output=MaaCustomControlUnitCreate(c.has_table?&c.table:nullptr,c.manager.controller_arg_);
        if(*output)++unit_counts[0];return 0;});
}
int32_t mfs_custom_unit_rust_create(MfsCustomUnitDispatch dispatch,MfsCustomUnitDrop drop,void* argument,void** output) noexcept {
    if(!output)return 1;*output=nullptr;if(!dispatch || !drop)return 1;
    return guarded([&]{*output=static_cast<MaaCustomControlUnitHandle>(new RustUnit(dispatch,drop,argument));++unit_counts[2];return 0;});
}
int32_t mfs_custom_unit_destroy(uint8_t rust,void* unit) noexcept {
    if(rust>1)return 1;
    return guarded([&]{MaaCustomControlUnitDestroy(static_cast<MaaCustomControlUnitHandle>(unit));
        if(unit)++unit_counts[rust?3:1];return 0;});
}
uint32_t mfs_custom_unit_bases(void* unit) noexcept {
    if(!unit)return 0;auto* custom=static_cast<MaaCustomControlUnitHandle>(unit);
    MAA_CTRL_UNIT_NS::ControlUnitAPI* base=custom;
    return (dynamic_cast<MAA_CTRL_UNIT_NS::CustomControlUnitAPI*>(base)==custom?1:0)
        |(dynamic_cast<MAA_CTRL_UNIT_NS::ScrollableUnit*>(base)?2:0)
        |(dynamic_cast<MAA_CTRL_UNIT_NS::RelativeMovableUnit*>(base)?4:0)
        |(dynamic_cast<MAA_CTRL_UNIT_NS::ShellableUnit*>(base)?8:0);
}
int32_t mfs_custom_unit_query(void* unit,uint32_t op,void* output,uint64_t* value) noexcept {
    if(!unit || !value || op>6 || (op>=4 && !output) || (op<4 && output))return 1;*value=0;
    try {
        MAA_CTRL_UNIT_NS::ControlUnitAPI* base=static_cast<MaaCustomControlUnitHandle>(unit);
        switch(op){
        case 0:*value=base->connect();break;
        case 1:*value=base->connected();break;
        case 2:*value=base->inactive();break;
        case 3:*value=base->get_features();break;
        case 4:*value=base->request_uuid(static_cast<UuidOutput*>(output)->value);break;
        case 5:*value=base->screencap(static_cast<Image*>(output)->value);break;
        case 6:static_cast<Info*>(output)->value=base->get_info();*value=1;break;
        }
        return 0;
    }catch(const RustUnitError& error){return error.code;}
    catch(const cv::Exception&){return 2;}
    catch(const std::exception&){return 3;}catch(...){return 4;}
}
uint64_t mfs_custom_unit_count(uint32_t index) noexcept {
    return index<std::size(unit_counts)?unit_counts[index].load():UINT64_MAX;
}
int32_t mfs_custom_unit_fault(void* context,uint32_t operation) noexcept {
    if(!context || operation>6)return 1;auto& table=static_cast<Context*>(context)->table;
    switch(operation){
    case 0:table.connect=unit_bool_fault;break;
    case 1:table.connected=unit_bool_fault;break;
    case 2:table.inactive=unit_bool_fault;break;
    case 3:table.get_features=features_fault;break;
    case 4:table.request_uuid=uuid_fault;break;
    case 5:table.screencap=native_fault;break;
    case 6:table.get_info=uuid_fault;break;
    }
    return 0;
}
int32_t mfs_custom_unit_publish_image(void* destination,void* image) noexcept {
    if(!destination || !image || active_image_scopes!=1)return 1;
    return guarded([&]{
        // The Mat head was obtained from this live ImageBuffer before entering
        // this function. Move its real storage to the actual caller now, so no
        // extra native pixel lease remains when that buffer is destroyed.
        *static_cast<cv::Mat*>(destination)=std::move(static_cast<Image*>(image)->value);
        ++unit_counts[6];return 0;});
}
int32_t mfs_custom_unit_publish_uuid(void* destination,MaaStringBuffer* buffer) noexcept {
    if(!destination || !buffer || active_string_scopes!=1)return 1;
    return guarded([&]{
        // Same original std::string assignment, including reuse/empty/NUL
        // semantics, while the original temporary buffer is still alive.
        *static_cast<std::string*>(destination)=buffer->get();++unit_counts[5];return 0;});
}
uint32_t mfs_custom_unit_publication_probe() noexcept {
    try {
        cv::Mat destination;Image image(cv::Mat{});std::string uuid;MAA_NS::StringBuffer buffer;
        return (mfs_custom_unit_publish_image(&destination,&image)==1?1:0)
            |(mfs_custom_unit_publish_uuid(&uuid,&buffer)==1?2:0);
    }catch(...){return 0;}
}
