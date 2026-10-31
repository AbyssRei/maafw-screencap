// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "../../fixtures/pipewire-format-pattern.h"
// Compositional oracle only: use the actual F table/OpenCV on owned fixture
// bytes. Live source capture below still invokes the unedited original class.
struct FormatStream {
    std::array<cv::Mat,2> expected;
    std::array<size_t,2> seen{};
    std::ofstream records;
    std::string name;
    explicit FormatStream(std::string_view selected,const std::filesystem::path& root):name(selected) {
        const std::array<std::pair<std::string_view,spa_video_format>,7> formats={{{"RGB",SPA_VIDEO_FORMAT_RGB},
            {"RGBA",SPA_VIDEO_FORMAT_RGBA},{"RGBx",SPA_VIDEO_FORMAT_RGBx},{"BGR",SPA_VIDEO_FORMAT_BGR},
            {"BGRA",SPA_VIDEO_FORMAT_BGRA},{"BGRx",SPA_VIDEO_FORMAT_BGRx},{"YUY2",SPA_VIDEO_FORMAT_YUY2}}};
        const auto selected_format=std::find_if(formats.begin(),formats.end(),[&](const auto& f){return f.first==selected;});
        require(selected_format!=formats.end(),"unknown explicit stream format");
        bool found=false;
        std::filesystem::create_directories(root/"oracle");
        for(const auto& info:MfsOriginalPipeWire::kSpaFormatTable) {
            if(info.format!=selected_format->second) continue;
            found=true;
            const auto stride=static_cast<size_t>(WIDTH*info.channels+12);
            for(unsigned phase=0;phase<2;++phase) {
                std::vector<uint8_t> bytes(stride*HEIGHT);
                mfs_pw_format_pattern(bytes.data(),static_cast<size_t>(info.channels),stride,phase);
                cv::Mat borrowed(HEIGHT,WIDTH,CV_8UC(info.channels),bytes.data(),stride),owned=borrowed.clone();
                if(info.cv_conversion>=0) cv::cvtColor(owned,expected[phase],info.cv_conversion);
                else expected[phase]=std::move(owned);
                save_bytes(root/"oracle"/("phase-"+std::to_string(phase)+".raw"),bytes.data(),bytes.size());
                save(expected[phase],root/"oracle"/("phase-"+std::to_string(phase)+".bgr"));
            }
        }
        require(found,"stream format missing from original table");
        records.open(root/"calls.tsv");
        records<<"cycle\tcall\tphase\n";
        require(records.good(),"cannot create format stream records");
    }
    unsigned phase(const cv::Mat& image) const {
        require(image.type()==CV_8UC3 && image.cols==WIDTH && image.rows==HEIGHT && image.isContinuous() && image.step[0]==84,
                "format stream target layout differs");
        for(unsigned p=0;p<2;++p)
            if(std::memcmp(image.data,expected[p].data,2016)==0) return p;
        throw std::runtime_error("format stream BGR differs from both known phases");
    }
    unsigned observe(const cv::Mat& image,int cycle,int call) {
        const auto value=phase(image);
        records<<cycle<<'\t'<<call<<'\t'<<value<<'\n';records.flush();
        require(records.good(),"cannot save format stream row");
        ++seen[value];return value;
    }
    void finish() {
        records.close();require(records.good() && seen[0]>0 && seen[1]>0 && seen[0]+seen[1]==60,
            "format stream phases or records incomplete");
        std::cout<<"original PipeWire format="<<name<<" cycles=20 captures=60 retained=60 phases=2 passed\n";
    }
};
