// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included in the original-source observation TU after its resource helpers.
void protocol_context(const std::filesystem::path& socket,const std::filesystem::path& directory,unsigned version)
{
    // Fixture-only conversion initialization, outside capture/resource counts.
    cv::Mat warm4(5,7,CV_8UC4,cv::Scalar(1,2,3,4)),warm3(5,7,CV_8UC3,cv::Scalar(1,2,3)),converted;
    cv::cvtColor(warm4,converted,cv::COLOR_RGBA2BGR);
    cv::cvtColor(warm3,converted,cv::COLOR_RGB2BGR);warm3.copyTo(converted);
    const auto baseline_fds=count("/proc/self/fd");
    const auto baseline_tasks=tasks();
    std::vector<cv::Mat> held;
    std::vector<std::vector<uint8_t>> expected;
    std::ostringstream rows;
    for(unsigned cycle=0;cycle<20;++cycle) {
        {
            auto client=std::make_shared<MfsOriginalWlr::WaylandClient>();
            require(client->connect(socket),"source protocol matrix connect failed");
            require(zwlr_screencopy_manager_v1_get_version(client->get_screencopy_manager().get())==version,
                "source protocol matrix manager version differs");
            MfsOriginalWlr::WlrScreencap source(client);
            for(unsigned index=0;index<24;++index) {
                std::cout<<"original protocol case="<<cycle<<'-'<<index<<" version="<<version<<'\n';
                const auto input=mfs_protocol_case(index);
                std::array<uint8_t,205> native{};
                mfs_protocol_fill(input,native.data());
                std::vector<uint8_t> legacy(7*5*3),target(7*5*3);
                const bool swap=index/4==2 || index/4==3 || index/4==5;
                for(unsigned y=0;y<5;++y) { for(unsigned x=0;x<7;++x) {
                    const auto pixel=y*7+x;
                    for(unsigned channel=0;channel<3;++channel) {
                        legacy[pixel*3+channel]=native[pixel*input.channels+(swap ? 2-channel : channel)];
                        target[pixel*3+channel]=mfs_protocol_color(x,y,2-channel,index/4);
                    }
                }}
                cv::Mat image;
                require(source.screencap(image),"source protocol matrix capture failed");
                require(image.type()==CV_8UC3 && image.cols==7 && image.rows==5,"source protocol image layout");
                std::vector<uint8_t> actual;
                for(int y=0;y<5;++y) { actual.insert(actual.end(),image.ptr(y),image.ptr(y)+21); }
                const bool target_matches=actual==target;
                const auto name="case-"+std::to_string(cycle)+"-"+std::to_string(index)+".source.bgr";
                std::ofstream file(directory/name,std::ios::binary);
                file.write(reinterpret_cast<const char*>(actual.data()),static_cast<std::streamsize>(actual.size()));
                file.close();require(file.good(),"cannot save source protocol pixels");
                require(actual==legacy,"source packed conversion bytes differ");
                require(target_matches==(index%4==0),"unexpected stride/YInvert source target relation");
                rows<<cycle<<'\t'<<index<<'\t'<<version<<'\t'<<input.format<<'\t'<<input.stride<<'\t'
                    <<input.inverted<<'\t'<<target_matches<<'\n';
                held.push_back(std::move(image));expected.push_back(std::move(legacy));
            }
        }
        require(count("/proc/self/fd")==baseline_fds && tasks()==baseline_tasks,"source protocol resources differ");
        std::cout<<"original protocol cycle="<<cycle<<" held="<<held.size()<<" fds="<<baseline_fds<<" balanced=1\n";
    }
    std::ofstream table(directory/"source-cases.tsv");
    table<<rows.str();table.close();require(table.good(),"cannot save source protocol case rows");
    for(size_t index=0;index<held.size();++index) {
        for(int y=0;y<5;++y) {
            require(std::equal(held[index].ptr(y),held[index].ptr(y)+21,expected[index].data()+y*21),
                "source protocol held pixels mutated");
        }
    }
    std::cout<<"original Wlr protocol version="<<version<<" cycles=20 captures=480 retained=480 formats=6 layouts=4 resources_balanced=1 legacy_packed=1 passed\n";
}
