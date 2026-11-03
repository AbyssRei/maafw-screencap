// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Explicit native acceptance only. The original C API implementations compile
// in their complete, unchanged F translation unit, never in this adapter.
#include <algorithm>
size_t mfs_custom_buffer_abi(uint32_t index) noexcept {
    const size_t values[]{1,sizeof(void*),sizeof(MaaSize),sizeof(MaaBool),sizeof(int32_t),sizeof(uint64_t)};
    return index<std::size(values)?values[index]:SIZE_MAX;
}
int32_t mfs_custom_string_buffer_probe(const uint8_t* data,size_t length,uint64_t* v,size_t count,void** output) noexcept {
    if(!output)return 1;*output=nullptr;
    if(!v || count!=24 || (!data && length) || length>64)return 1;
    std::fill_n(v,count,0);
    return guarded([&]{
        const char empty=0;const auto* input=data?reinterpret_cast<const char*>(data):&empty;
        std::unique_ptr<MaaStringBuffer,decltype(&MaaStringBufferDestroy)> buffer(MaaStringBufferCreate(),MaaStringBufferDestroy);
        v[0]=MaaStringBufferIsEmpty(buffer.get());v[1]=MaaStringBufferSize(buffer.get());v[2]=MaaStringBufferGet(buffer.get())!=nullptr;
        v[3]=MaaStringBufferIsEmpty(nullptr);v[4]=MaaStringBufferClear(nullptr);
        const auto* absent=MaaStringBufferGet(nullptr);v[5]=absent!=nullptr;v[6]=absent && *absent==0;
        v[7]=MaaStringBufferSize(nullptr);v[8]=MaaStringBufferSet(nullptr,"x");v[9]=MaaStringBufferSetEx(nullptr,"x",1);
        MaaStringBufferDestroy(nullptr);
        v[10]=MaaStringBufferSet(buffer.get(),"warmup");v[11]=MaaStringBufferSize(buffer.get());
        v[12]=MaaStringBufferSetEx(buffer.get(),input,static_cast<MaaSize>(length));
        v[13]=MaaStringBufferSize(buffer.get());v[14]=MaaStringBufferIsEmpty(buffer.get());
        v[15]=MaaStringBufferSet(buffer.get(),nullptr);v[16]=MaaStringBufferSetEx(buffer.get(),nullptr,0);
        v[17]=MaaStringBufferSize(buffer.get());
        // Diagnostic string copy while the actual native buffer is live.
        auto held=std::make_unique<UuidOutput>(std::string(MaaStringBufferGet(buffer.get()),static_cast<size_t>(MaaStringBufferSize(buffer.get()))));
        v[18]=MaaStringBufferClear(buffer.get());v[19]=MaaStringBufferIsEmpty(buffer.get());v[20]=MaaStringBufferSize(buffer.get());
        const auto* cleared=MaaStringBufferGet(buffer.get());v[21]=cleared && *cleared==0;v[22]=held->value.size();
        buffer.reset();v[23]=1;*output=held.release();return 0;
    });
}
int32_t mfs_custom_image_buffer_probe(const uint8_t* data,size_t length,int32_t type,uint64_t* v,size_t count,void** output) noexcept {
    if(!output)return 1;*output=nullptr;
    if(!v || count!=32 || !data || !length || length>1024 || type<0)return 1;
    std::fill_n(v,count,0);
    int32_t canonical=-1;size_t scalar=0,element=0;
    if(mfs_custom_raw_type(CV_MAT_DEPTH(type),CV_MAT_CN(type),&canonical,&scalar,&element) || canonical!=type || length!=element*6)return 1;
    return guarded([&]{
        std::unique_ptr<MaaImageBuffer,decltype(&MaaImageBufferDestroy)> buffer(MaaImageBufferCreate(),MaaImageBufferDestroy);
        v[0]=MaaImageBufferIsEmpty(buffer.get());v[1]=MaaImageBufferGetRawData(buffer.get())==nullptr;
        v[2]=MaaImageBufferWidth(buffer.get());v[3]=MaaImageBufferHeight(buffer.get());v[4]=MaaImageBufferChannels(buffer.get());v[5]=MaaImageBufferType(buffer.get());
        v[6]=MaaImageBufferSetRawData(buffer.get(),const_cast<uint8_t*>(data),3,2,type);
        v[7]=MaaImageBufferIsEmpty(buffer.get());v[8]=MaaImageBufferWidth(buffer.get());v[9]=MaaImageBufferHeight(buffer.get());
        v[10]=MaaImageBufferChannels(buffer.get());v[11]=MaaImageBufferType(buffer.get());
        const auto* raw=MaaImageBufferGetRawData(buffer.get());v[12]=raw!=nullptr;v[13]=raw!=data;v[14]=raw==buffer->get().data;
        v[15]=MaaImageBufferIsEmpty(nullptr);v[16]=MaaImageBufferClear(nullptr);v[17]=MaaImageBufferGetRawData(nullptr)==nullptr;
        v[18]=MaaImageBufferWidth(nullptr);v[19]=MaaImageBufferHeight(nullptr);v[20]=MaaImageBufferChannels(nullptr);v[21]=MaaImageBufferType(nullptr);
        v[22]=MaaImageBufferSetRawData(nullptr,const_cast<uint8_t*>(data),3,2,type);MaaImageBufferDestroy(nullptr);
        // A real Mat head shares the independent native clone. No second pixel
        // copy is added here, and no raw pointer outlives its real storage owner.
        auto held=std::make_unique<Image>(buffer->get());
        v[23]=MaaImageBufferClear(buffer.get());v[24]=MaaImageBufferIsEmpty(buffer.get());v[25]=MaaImageBufferGetRawData(buffer.get())==nullptr;
        v[26]=MaaImageBufferWidth(buffer.get());v[27]=MaaImageBufferHeight(buffer.get());v[28]=MaaImageBufferChannels(buffer.get());v[29]=MaaImageBufferType(buffer.get());
        v[30]=held->value.total()*held->value.elemSize();buffer.reset();v[31]=1;*output=held.release();return 0;
    });
}
