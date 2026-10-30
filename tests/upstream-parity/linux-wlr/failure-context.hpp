// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included after source-context helpers; original capture/client code is unchanged.
void failure_context(const std::filesystem::path& socket,const std::filesystem::path& directory)
{
    const auto baseline_fds=count("/proc/self/fd");
    const auto baseline_tasks=tasks();
    std::vector<cv::Mat> held;
    std::vector<unsigned> held_indices;
    std::ostringstream rows,resources;
    for(unsigned cycle=0;cycle<20;++cycle) {
        {
            auto client=std::make_shared<MfsOriginalWlr::WaylandClient>();
            require(client->connect(socket),"source failure fixture connect failed");
            MfsOriginalWlr::WlrScreencap source(client);
            cv::Mat output(5,7,CV_8UC3,cv::Scalar(201,73,17));
            for(unsigned phase=0;phase<5;++phase) {
                // The host detaches before the next successful write, so the
                // previous success really remains owned through later SHM reuse.
                if(phase==2) { held.push_back(std::move(output));held_indices.push_back(0);output=held.back(); }
                if(phase==3) { output.release(); }
                const auto* before=output.data;
                const auto owners=output.u ? output.u->refcount : 0;
                const auto result=source.screencap(output);
                const bool expected=phase==1 || phase==3;
                require(result==expected,"source scripted failure result differs");
                require(client->connected(),"original process_requests failure cleared connected flag");
                if(!expected) {
                    require(output.data==before && output.u && output.u->refcount==owners,
                        "source failed request replaced caller image owner");
                }
                require(output.type()==CV_8UC3 && output.cols==7 && output.rows==5,"source failure output layout");
                const unsigned index=phase<3 ? 0 : 4;
                std::ofstream file(directory/("case-"+std::to_string(cycle)+"-"+std::to_string(phase)+".source.bgr"),std::ios::binary);
                for(unsigned y=0;y<5;++y) { for(unsigned x=0;x<7;++x) {
                    const auto pixel=output.at<cv::Vec3b>(static_cast<int>(y),static_cast<int>(x));
                    for(unsigned channel=0;channel<3;++channel) {
                        const uint8_t target=phase==0 ? std::array<uint8_t,3>{201,73,17}[channel]
                            : mfs_protocol_color(x,y,2-channel,index/4);
                        require(pixel[static_cast<int>(channel)]==target,"source failure output bytes changed");
                    }
                } }
                for(int y=0;y<5;++y) { file.write(reinterpret_cast<const char*>(output.ptr(y)),21); }
                file.close();require(file.good(),"cannot save source failure output");
                rows<<cycle<<'\t'<<phase<<'\t'<<result<<'\t'<<client->connected()<<'\t'<<(!expected)<<'\n';
            }
            held.push_back(std::move(output));held_indices.push_back(4);
        }
        const auto actual_fds=count("/proc/self/fd");
        const auto actual_tasks=tasks();
        require(actual_fds==baseline_fds && actual_tasks==baseline_tasks,"source failure native resources differ");
        resources<<cycle<<'\t'<<baseline_fds<<'\t'<<actual_fds<<'\t'<<baseline_tasks.size()<<'\t'<<actual_tasks.size()<<'\n';
    }
    for(size_t i=0;i<held.size();++i) {
        std::ofstream file(directory/("held-"+std::to_string(i)+".source.bgr"),std::ios::binary);
        for(unsigned y=0;y<5;++y) { for(unsigned x=0;x<7;++x) { for(unsigned c=0;c<3;++c) {
            require(held[i].at<cv::Vec3b>(static_cast<int>(y),static_cast<int>(x))[static_cast<int>(c)]
                ==mfs_protocol_color(x,y,2-c,held_indices[i]/4),"source retained failure image mutated");
        } } }
        for(int y=0;y<5;++y) { file.write(reinterpret_cast<const char*>(held[i].ptr(y)),21); }
        file.close();require(file.good(),"cannot save held source failure output");
    }
    std::ofstream table(directory/"source-cases.tsv");table<<rows.str();table.close();require(table.good(),"save source failure rows");
    std::ofstream resource_file(directory/"source-resources.tsv");resource_file<<resources.str();resource_file.close();
    require(resource_file.good(),"save source failure resource rows");
    std::cout<<"original Wlr failures cycles=20 operations=100 ready=40 failed_events=40 disconnects=20 false_output_preserved=60 connected_retained=20 held=40 fd_tasks_balanced=1 passed\n";
}
