// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included in the source fixture's anonymous namespace after its shared helpers.
void nul_names(const std::filesystem::path& socket,const std::filesystem::path& directory) {
    const auto first=socket.string()+std::string("\0first",6);
    const auto second=socket.string()+std::string("\0second",7);
    const std::filesystem::path leading(std::string("\0mfs-unused",11));
    require(first!=second && std::filesystem::path(first)!=std::filesystem::path(second),
        "source NUL suffixes do not distinguish full filesystem paths");
    const auto baseline_fds=count("/proc/self/fd");
    const auto baseline_tasks=tasks();
    std::vector<cv::Mat> held;held.reserve(60);
    for(int cycle=0;cycle<20;++cycle) {
        {
            auto client=std::make_shared<MfsOriginalWlr::WaylandClient>();
            require(client->connect(first),"original embedded-NUL native connect failed");
            const auto before=roundtrips;
            require(client->connect(first) && roundtrips==before,
                "original identical full NUL path was not a no-op");
            for(int call=0;call<2;++call) {
                if(call==1) {
                    const auto previous=roundtrips;
                    require(client->connect(second) && roundtrips>previous,
                        "original changed NUL suffix did not reopen native connection");
                }
                // No old capture/proxy snapshot survives the next reconnect.
                MfsOriginalWlr::WlrScreencap source(client);
                cv::Mat image;require(source.screencap(image),"original NUL-name capture failed");
                pixels(image);save(image,directory/("nul-"+std::to_string(cycle)+"-"+std::to_string(call)+".bgr"));
                held.push_back(std::move(image));
            }
            require(!client->connect(leading) && !client->connected() && !client->get_output(),
                "original leading NUL unexpectedly selected default socket");
            require(!client->connect({}) && !client->connected() && !client->get_output(),
                "original empty name unexpectedly selected default socket");
        }
        {
            MfsOriginalWlr::LinuxControlUnitConfig config;
            config.screencap_method=MaaLinuxScreencapMethod_Wlr;config.wlr_socket_path=first;
            MfsOriginalWlr::LinuxControlUnitMgr manager(config);
            std::string uuid;require(manager.request_uuid(uuid) && uuid==first+"_-1",
                "original manager UUID truncated its NUL suffix");
            require(manager.connect() && manager.connected(),"original NUL manager connect failed");
            cv::Mat image;require(manager.screencap(image),"original NUL manager capture failed");
            pixels(image);save(image,directory/("nul-manager-"+std::to_string(cycle)+".bgr"));
            held.push_back(std::move(image));
        }
        require(count("/proc/self/fd")==baseline_fds && tasks()==baseline_tasks,
            "original NUL-name resources did not return to baseline");
        std::cout<<"original NUL-name cycle="<<cycle<<" held="<<held.size()
            <<" fds="<<baseline_fds<<" resources_balanced=1\n";
    }
    for(const auto& image:held) { pixels(image); }
    std::cout<<"original Wlr nul_names cycles=20 captures=60 retained=60 same_full_name_noop=20 suffix_reconnect=20 leading_nul_rejected=20 empty_rejected=20 manager_full_uuid=20 resources_balanced=1 passed\n";
}
