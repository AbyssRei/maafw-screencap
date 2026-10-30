// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included only by the explicit original DesktopDup acceptance executable.
struct DesktopGeometryCase {
    std::string name;
    uint32_t style;
    std::array<int64_t,4> client;
};
using DesktopGeometrySnapshot=std::array<int64_t,31>;
void geometry_require(bool value,const char* message) {if(!value){throw std::runtime_error(message);}}
DesktopGeometrySnapshot desktop_geometry(Fixture& fixture,std::array<int64_t,6> input={}) {
    DesktopGeometrySnapshot row{};
    geometry_require(mfs_print_thread_geometry(fixture.value.owner,input.data(),input.size(),row.data(),row.size())==0,"native geometry fixture command failed");
    return row;
}
std::array<int64_t,4> desktop_client(const DesktopGeometrySnapshot& row) {
    return {row[16]+row[12],row[17]+row[13],row[16]+row[14],row[17]+row[15]};
}
std::array<int64_t,4> desktop_target(const DesktopGeometrySnapshot& row) {
    const auto client=desktop_client(row);
    if ((row[18]&WS_POPUP)!=0 && std::equal(client.begin(),client.end(),row.begin())) {return client;}
    const auto width=std::min(client[2]-client[0],row[6]-row[4]);
    const auto height=std::min(client[3]-client[1],row[7]-row[5]);
    geometry_require(width>=16 && height>=16,"native work-area matrix target too small");
    const auto left=std::clamp(client[0],row[4],row[6]-width),top=std::clamp(client[1],row[5],row[7]-height);
    return {left,top,left+width,top+height};
}
std::vector<DesktopGeometryCase> desktop_geometry_cases(const DesktopGeometrySnapshot& initial) {
    const std::array<int64_t,4> monitor{initial[0],initial[1],initial[2],initial[3]},work{initial[4],initial[5],initial[6],initial[7]};
    const uint32_t popup=WS_POPUP|WS_VISIBLE,ordinary=WS_OVERLAPPEDWINDOW|WS_VISIBLE;
    std::vector<DesktopGeometryCase> cases{{"exact_popup",popup,monitor}};
    for(int edge=0;edge<4;++edge) {for(int delta:{-9,-8,-7,-1,1,7,8,9}) {
        auto client=monitor;client[static_cast<size_t>(edge)]+=delta;
        cases.push_back({"edge_"+std::to_string(edge)+"_"+std::to_string(delta),popup,client});
    }}
    cases.push_back({"exact_bordered_popup",popup|WS_THICKFRAME,monitor});
    cases.push_back({"ordinary_monitor",ordinary,monitor});
    cases.push_back({"ordinary_work",ordinary,work});
    cases.push_back({"review_negative_one_translation",popup,{monitor[0]-1,monitor[1],monitor[2]-1,monitor[3]}});
    cases.push_back({"oversized_popup",popup,{monitor[0]-9,monitor[1]-9,monitor[2]+9,monitor[3]+9}});
    return cases;
}
void desktop_geometry_row(std::ofstream& output,size_t index,const char* stage,const DesktopGeometrySnapshot& row) {
    output<<index<<"\tsource\t"<<stage;
    for(auto value:row){output<<'\t'<<value;}output<<'\n';output.flush();
}
std::ofstream desktop_geometry_file(const std::filesystem::path& path) {
    std::ofstream file(path,std::ios::binary);file.exceptions(std::ios::failbit|std::ios::badbit);return file;
}
int desktop_geometry_matrix(const std::filesystem::path& output) {
    geometry_require(std::filesystem::is_empty(output),"desktop geometry output must be fresh");
    Fixture control;
    geometry_require(mfs_print_thread_fixture(96,64,0,0,0,0,MFS_RENDER_DISABLE_TRANSITIONS|MFS_RENDER_TOPMOST,&control.value)==0,"matrix control fixture creation failed");
    const auto initial=desktop_geometry(control);
    geometry_require(control.close(),"matrix control fixture did not close");
    if(std::equal(initial.begin(),initial.begin()+4,initial.begin()+4)) {
        std::cout<<"MFS_MISSING_ENVIRONMENT:desktop_geometry_workarea"<<std::endl;return 77;
    }
    const auto cases=desktop_geometry_cases(initial);
    geometry_require(cases.size()==38,"incomplete original desktop geometry definitions");
    auto definitions=desktop_geometry_file(output/"source-cases.tsv");
    auto snapshots=desktop_geometry_file(output/"source-snapshots.tsv");
    auto resources=desktop_geometry_file(output/"source-resources.tsv");
    auto layouts=desktop_geometry_file(output/"source-native-layouts.tsv");
    auto markers=desktop_geometry_file(output/"source-markers.tsv");
    for(size_t index=0;index<cases.size();++index) {
        const auto& c=cases[index];definitions<<index<<'\t'<<c.name<<'\t'<<c.style;
        for(auto value:c.client){definitions<<'\t'<<value;}definitions<<'\n';definitions.flush();
        std::cout<<"original DD geometry begin="<<index<<" name="<<c.name<<std::endl;
        Fixture fixture;
        geometry_require(mfs_print_thread_fixture(96,64,0,0,0,0,MFS_RENDER_DISABLE_TRANSITIONS|MFS_RENDER_TOPMOST,&fixture.value)==0,"matrix window fixture creation failed");
        const auto before=desktop_geometry(fixture,{1,c.style,c.client[0],c.client[1],c.client[2]-c.client[0],c.client[3]-c.client[1]});
        desktop_geometry_row(snapshots,index,"setup",before);
        geometry_require(desktop_client(before)==c.client,"original matrix native setup client differs");
        geometry_require(std::equal(before.begin(),before.begin()+8,initial.begin()),"original matrix monitor/work area changed");
        const auto target=desktop_target(before);
        geometry_require(MaaNS::CtrlUnitNs::ensure_window_on_screen(static_cast<HWND>(fixture.value.hwnd)),"original matrix helper failed");
        const auto until=std::chrono::steady_clock::now()+std::chrono::seconds(3);
        auto corrected=desktop_geometry(fixture);
        while(desktop_client(corrected)!=target) {
            geometry_require(std::chrono::steady_clock::now()<until,"original matrix asynchronous geometry did not complete");
            Sleep(1);corrected=desktop_geometry(fixture);
        }
        desktop_geometry_row(snapshots,index,"corrected",corrected);
        geometry_require(corrected[20]==before[20]+static_cast<int64_t>(target!=c.client),"original matrix correction message count differs");
        desktop_geometry(fixture,{2,0,0,0,0,0});
        geometry_require(mfs_desktop_fixture_inside_updates(fixture.value.owner)==0,"matrix in-target marker failed");
        desktop_geometry_row(snapshots,index,"presented",desktop_geometry(fixture));
        const auto baseline=GetGuiResources(GetCurrentProcess(),GR_GDIOBJECTS);
        std::vector<cv::Mat> held;held.reserve(2);
        {
            MaaNS::CtrlUnitNs::DesktopDupWindowScreencap capture(static_cast<HWND>(fixture.value.hwnd));
            for(int call=0;call<2;++call) {
                auto image=capture.screencap();
                geometry_require(image.has_value() && !image->empty(),"original matrix capture returned no image");
                geometry_require(image->cols==target[2]-target[0] && image->rows==target[3]-target[1],"original matrix crop dimensions differ");
                const auto name=std::to_string(index)+"-source-"+std::to_string(call);
                save(*image,output/(name+".bgr"));
                const auto row=static_cast<uint64_t>(image->cols)*3,span=row*static_cast<uint64_t>(image->rows);
                auto shape=desktop_geometry_file(output/(name+".txt"));shape<<image->cols<<'\t'<<image->rows<<'\t'<<row<<'\t'<<span<<'\n';
                // Original ROI retains full-output Mat step. Saved rows are
                // packed independently; never relabel this native step as row.
                layouts<<index<<'\t'<<call<<'\t'<<image->cols<<'\t'<<image->rows<<'\t'<<image->step[0]<<'\t'<<row<<'\t'<<span<<'\n';layouts.flush();
                geometry_require(known(*image,true),"original matrix known target/marker pixel mismatch");
                const auto after=desktop_geometry(fixture);
                desktop_geometry_row(snapshots,index,("capture-"+std::to_string(call)).c_str(),after);
                geometry_require(desktop_client(after)==target && after[20]==corrected[20],"original matrix capture repeated geometry correction");
                held.push_back(std::move(*image));
            }
        }
        uint64_t marker[3]{};
        geometry_require(mfs_desktop_fixture_update_info(fixture.value.owner,marker,3)==0 && marker[0]==reinterpret_cast<uintptr_t>(fixture.value.hwnd)
            && marker[1]>=1 && marker[2]==0,"original matrix marker ownership/presentation differs");
        markers<<index<<"\tsource\t"<<marker[0]<<'\t'<<marker[1]<<'\t'<<marker[2]<<'\n';markers.flush();
        geometry_require(fixture.close(),"original matrix fixture close failed");
        const auto closed=GetGuiResources(GetCurrentProcess(),GR_GDIOBJECTS);
        resources<<index<<"\tsource\t"<<baseline<<'\t'<<closed<<'\n';resources.flush();
        geometry_require(closed<=baseline,"original matrix fixture GDI owners remain after close");
        for(size_t call=0;call<held.size();++call) {
            geometry_require(known(held[call],true),"original matrix held CPU target changed after close");
            save(held[call],output/(std::to_string(index)+"-source-"+std::to_string(call)+".held.bgr"));
        }
        std::cout<<"original DD geometry case="<<index<<" name="<<c.name<<" captures=2 retained=2 passed"<<std::endl;
    }
    std::cout<<"original DesktopDupWindow geometry cases=38 captures=76 retained_after_close=76 known_target_with_marker=1 repeated_geometry=0 native_roi_stride_recorded=1 passed"<<std::endl;
    return 0;
}
