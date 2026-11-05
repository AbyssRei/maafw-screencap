// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "MaaUtils/Uuid.h"
#include <fstream>
#include <iostream>
#include <set>
#include <stdexcept>

int main(int argc,char** argv) {
    try {
        if(argc!=2) { throw std::runtime_error("usage: uuid-corpus <new output file>"); }
        // The preparation caller owns a new external directory. Persist each
        // real draw; random outputs are not expected to match across processes.
        std::ofstream output(argv[1],std::ios::binary);
        if(!output) { throw std::runtime_error("cannot create UUID corpus"); }
        std::set<std::string> seen;
        for(unsigned draw=0;draw<256;++draw) {
            const auto uuid=MaaNS::make_uuid();
            if(uuid.size()!=36) { throw std::runtime_error("original UUID length differs"); }
            for(size_t i=0;i<uuid.size();++i) {
                const bool separator=i==8 || i==13 || i==18 || i==23;
                const char c=uuid[i];
                if(separator ? c!='-' : !((c>='0' && c<='9') || (c>='a' && c<='f'))) {
                    throw std::runtime_error("original UUID spelling differs");
                }
            }
            if(uuid[14]!='4' || (uuid[19]!='8' && uuid[19]!='9' && uuid[19]!='a' && uuid[19]!='b') ||
               !seen.insert(uuid).second) { throw std::runtime_error("original UUID version/variant/uniqueness differs"); }
            output<<uuid<<'\n';
        }
        output.close();
        if(!output) { throw std::runtime_error("cannot save UUID corpus"); }
        std::cout<<"original MaaUtils UUID draws=256 unique=256 canonical_v4=256 passed\n";
        return 0;
    }catch(const std::exception& error) { std::cerr<<error.what()<<'\n';return 1; }
}
