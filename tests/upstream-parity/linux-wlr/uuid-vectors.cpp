// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include <boost/uuid/basic_random_generator.hpp>
#include <boost/uuid/detail/chacha20.hpp>
#include <boost/uuid/uuid_io.hpp>
#include <array>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>

struct Seed {
    std::array<unsigned char,48> bytes{};
    unsigned calls=0,words=0;
    template<class Iter> void generate(Iter first,Iter last) {
        ++calls;
        for(;first!=last;++first) {
            if(words>=12) { throw std::runtime_error("unexpected Boost seed span"); }
            const auto offset=words++*4;
            *first=std::uint32_t(bytes[offset]) | (std::uint32_t(bytes[offset+1])<<8) |
                (std::uint32_t(bytes[offset+2])<<16) | (std::uint32_t(bytes[offset+3])<<24);
        }
    }
};
int main(int argc,char** argv) {
    try {
        if(argc!=2 || std::filesystem::exists(argv[1])) { throw std::runtime_error("usage: uuid-vectors <new file>"); }
        std::ofstream out(argv[1],std::ios::binary);
        constexpr char hex[]="0123456789abcdef";
        for(unsigned scenario=0;scenario<8;++scenario) {
            Seed seed;
            for(unsigned i=0;i<48;++i) {
                unsigned value=0;
                switch(scenario) {
                case 1:value=255;break;
                case 2:value=i;break;
                case 3:value=i%2 ? 0x55 : 0xaa;break;
                case 4:value=(i*73+19)&255;break;
                case 5:value=(i*i+127)&255;break;
                case 6:value=i>=32 && i<40 ? 255 : 0;break;
                case 7:value=i>=40 ? 255 : 0;break;
                default:break;
                }
                seed.bytes[i]=static_cast<unsigned char>(value);
            }
            boost::uuids::detail::chacha20_12 engine;
            engine.seed(seed);
            if(seed.calls!=1 || seed.words!=12) { throw std::runtime_error("Boost seed consumption differs"); }
            boost::uuids::basic_random_generator<boost::uuids::detail::chacha20_12> generator(engine);
            std::string seed_hex;
            for(auto b:seed.bytes) {seed_hex+=hex[b>>4];seed_hex+=hex[b&15];}
            for(unsigned draw=0;draw<256;++draw) {
                out<<scenario<<'\t'<<seed_hex<<'\t'<<draw<<"\tmaafw-wl-buffer-"<<boost::uuids::to_string(generator())<<'\n';
            }
            if(seed.calls!=1 || seed.words!=12) { throw std::runtime_error("Boost unexpectedly reseeded"); }
        }
        out.close();if(!out) {throw std::runtime_error("cannot save UUID vectors");}
        std::cout<<"original Boost UUID seeds=8 draws=2048 seed_words=96 reseeds=0 passed\n";
        return 0;
    }catch(const std::exception& error) {std::cerr<<error.what()<<'\n';return 1;}
}
