// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included after the source-context resource helpers; original code is unchanged.
void registry_context(const std::filesystem::path& socket,const std::filesystem::path& directory)
{
    const auto baseline_fds=count("/proc/self/fd");
    const auto baseline_tasks=tasks();
    std::vector<cv::Mat> held;
    std::vector<std::vector<uint8_t>> expected;
    std::ostringstream rows;
    {
        auto client=std::make_shared<MfsOriginalWlr::WaylandClient>();
        require(client->connect(socket),"source registry connect failed");
        for(unsigned round=0;round<20;++round) {
            const auto old_output=client->get_output();
            require(static_cast<bool>(old_output),"source initial output missing");
            MfsOriginalWlr::WlrScreencap old(client);
            const auto shot=[&](auto& source,const auto& output,unsigned phase) {
                const unsigned generation=round+(phase<2 ? 1 : 2);
                const unsigned index=generation%2 ? 0 : 4;
                const unsigned position=round*4+phase;
                cv::Mat image;
                require(source.screencap(image),"source registry capture failed");
                require(image.type()==CV_8UC3 && image.cols==7 && image.rows==5,"source registry image layout");
                std::vector<uint8_t> actual,target;
                for(unsigned y=0;y<5;++y) { for(unsigned x=0;x<7;++x) {
                    for(unsigned c=0;c<3;++c) { target.push_back(mfs_protocol_color(x,y,2-c,index/4)); }
                } }
                for(int y=0;y<5;++y) { actual.insert(actual.end(),image.ptr(y),image.ptr(y)+21); }
                std::ofstream file(directory/("case-"+std::to_string(position)+".source.bgr"),std::ios::binary);
                file.write(reinterpret_cast<const char*>(actual.data()),static_cast<std::streamsize>(actual.size()));
                file.close();require(file.good(),"cannot save source registry pixels");
                require(actual==target,"source registry selected wrong output pixels");
                auto* proxy=reinterpret_cast<wl_proxy*>(output.get());
                const auto version=wl_proxy_get_version(proxy);
                require(version==1+generation%2,"source did not bind advertised output version");
                rows<<position<<'\t'<<generation<<'\t'<<wl_proxy_get_id(proxy)
                    <<'\t'<<index<<'\t'<<version<<'\n';
                held.push_back(std::move(image));expected.push_back(std::move(target));
            };
            shot(old,old_output,0);
            require(client->process_requests(),"source registry addition dispatch failed");
            const auto next_output=client->get_output();
            require(next_output && next_output!=old_output,"source last-advertised output was not replaced");
            MfsOriginalWlr::WlrScreencap next(client);
            shot(old,old_output,1);
            shot(next,next_output,2);
            require(client->process_requests(),"source registry removal dispatch failed");
            require(client->get_output()==next_output,"source global_remove unexpectedly cleared output");
            MfsOriginalWlr::WlrScreencap after_remove(client);
            shot(after_remove,next_output,3);
        }
    }
    require(count("/proc/self/fd")==baseline_fds && tasks()==baseline_tasks,"source registry resources differ");
    for(size_t i=0;i<held.size();++i) {
        std::ofstream file(directory/("case-"+std::to_string(i)+".source.held.bgr"),std::ios::binary);
        for(int y=0;y<5;++y) {
            file.write(reinterpret_cast<const char*>(held[i].ptr(y)),21);
            require(std::equal(held[i].ptr(y),held[i].ptr(y)+21,expected[i].data()+y*21),"source registry held pixels mutated");
        }
        file.close();require(file.good(),"cannot save held source registry pixels");
    }
    std::ofstream table(directory/"source-cases.tsv");
    table<<rows.str();table.close();require(table.good(),"cannot save source registry table");
    const auto final_fds=count("/proc/self/fd");
    const auto final_tasks=tasks();
    std::ofstream resources(directory/"source-resources.tsv");
    resources<<baseline_fds<<'\t'<<final_fds<<'\t'<<baseline_tasks.size()<<'\t'<<final_tasks.size()<<'\n';
    resources.close();require(resources.good(),"cannot save source registry resources");
    require(count("/proc/self/fd")==baseline_fds,"source registry report owner not closed");
    std::cout<<"original Wlr registry rounds=20 captures=80 retained=80 replacement_snapshots=20 removed_output_retained=20 advertised_versions=1,2 resources_balanced=1 passed\n";
}
