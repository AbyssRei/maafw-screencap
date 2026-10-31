// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Self-owned PNG fixtures use the selected real codec. Rust independently
// computes the exact expected gray/BGR pixels and integer-ratio area results.
int32_t mfs_custom_codec_fixture(uint32_t channels,void** output) noexcept {
    if(!output)return 1;*output=nullptr;
    if(channels!=1 && channels!=3 && channels!=4)return 1;
    return guarded([&]{
        const int kind=channels==1?CV_8UC1:channels==3?CV_8UC3:CV_8UC4;
        cv::Mat image(6,8,kind);
        for(int y=0;y<image.rows;++y)for(int x=0;x<image.cols;++x){
            auto* pixel=image.ptr<uint8_t>(y)+static_cast<size_t>(x)*channels;
            if(channels==1){pixel[0]=static_cast<uint8_t>(12+8*x+4*y);continue;}
            pixel[0]=static_cast<uint8_t>(12+8*x);
            pixel[1]=static_cast<uint8_t>(20+8*y);
            pixel[2]=static_cast<uint8_t>(28+4*(x+y));
            if(channels==4)pixel[3]=static_cast<uint8_t>(40+5*x+3*y);
        }
        std::vector<uint8_t> bytes;
        if(!cv::imencode(".png",image,bytes))return 1;
        // This explicit fixture owner adds a test-only encoded-byte copy; it
        // is not a second capture pixel copy or an SDK encoded representation.
        *output=new UuidOutput(std::string(bytes.begin(),bytes.end()));return 0;
    });
}
int32_t mfs_custom_encoded_set(MaaImageBuffer* buffer,uint8_t* data,size_t length,uint8_t* result) noexcept {
    if(!result)return 1;*result=0;
    // A nonnull caller span is initialized and exclusively retained through
    // this call. Null is passed through to exercise the real C rejection.
    if(length>static_cast<size_t>(PTRDIFF_MAX))return 1;
    return guarded([&]{*result=MaaImageBufferSetEncoded(buffer,data,static_cast<MaaSize>(length));return 0;});
}
int32_t mfs_custom_encoded_view(MaaImageBuffer* buffer,MfsCustomStringView* output) noexcept {
    if(!output)return 1;*output={};
    return guarded([&]{
        const auto* bytes=MaaImageBufferGetEncoded(buffer);
        const auto length=MaaImageBufferGetEncodedSize(buffer);
        if(length>static_cast<MaaSize>(PTRDIFF_MAX) || (length && !bytes))return 1;
        // Both getters run on this owner channel; the first finishes lazy
        // encoding and the second reads the same unchanged owned cache.
        *output={bytes,static_cast<size_t>(length)};return 0;
    });
}
int32_t mfs_custom_resize(MaaImageBuffer* buffer,int32_t width,int32_t height,uint8_t* result) noexcept {
    if(!result)return 1;*result=0;
    return guarded([&]{*result=MaaImageBufferResize(buffer,width,height);return 0;});
}
int32_t mfs_custom_codec_decode(const uint8_t* data,size_t length,void** output) noexcept {
    if(!output)return 1;*output=nullptr;
    if(!readable(data,length))return 1;
    return guarded([&]{
        cv::Mat image;
        if(length){
            // Independent diagnostic decode of copied cache bytes, outside
            // the manager publication path. This test work is not eliminated.
            std::vector<uint8_t> bytes(data,data+length);
            image=cv::imdecode(bytes,cv::IMREAD_COLOR);
        }
        *output=new Image(std::move(image));return 0;
    });
}
