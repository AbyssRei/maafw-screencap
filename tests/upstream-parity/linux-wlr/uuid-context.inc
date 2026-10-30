// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
std::string make_uuid() {
    // External reference-only name provider. Every byte comes from getrandom;
    // version/variant and hyphen positions match the usual random UUID shape.
    std::array<unsigned char,16> bytes{};
    size_t offset=0;
    while(offset<bytes.size()) {
        const auto read=getrandom(bytes.data()+offset,bytes.size()-offset,0);
        if(read<0 && errno==EINTR) { continue; }
        require(read>0,"reference UUID getrandom failed");
        offset+=static_cast<size_t>(read);
    }
    bytes[6]=static_cast<unsigned char>((bytes[6]&15)|64);
    bytes[8]=static_cast<unsigned char>((bytes[8]&63)|128);
    constexpr char digits[]="0123456789abcdef";
    std::string result;result.reserve(36);
    for(size_t i=0;i<bytes.size();++i) {
        if(i==4 || i==6 || i==8 || i==10) { result.push_back('-'); }
        result.push_back(digits[bytes[i]>>4]);result.push_back(digits[bytes[i]&15]);
    }
    return result;
}
