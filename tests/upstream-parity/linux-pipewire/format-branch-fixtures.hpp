// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included inside the source probe's anonymous namespace, after save_bytes.
// Calls the unchanged F helper; no stream, callback, image or capture is created.
void save_format_branch_fixtures(const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    std::ofstream ids(directory/"ids.tsv");
    ids<<"bgra\tnv12\tunknown\n"<<SPA_VIDEO_FORMAT_BGRA<<'\t'<<SPA_VIDEO_FORMAT_NV12<<'\t'<<SPA_VIDEO_FORMAT_UNKNOWN<<'\n';
    ids.close();require(ids.good(),"cannot save parser branch IDs");
    struct Case { uint32_t first;bool second;uint32_t last;bool choice; };
    const Case cases[]={
        {SPA_VIDEO_FORMAT_BGRA,false,0,false},
        {SPA_VIDEO_FORMAT_NV12,false,0,false},
        {SPA_VIDEO_FORMAT_UNKNOWN,true,SPA_VIDEO_FORMAT_BGRA,false},
        {SPA_VIDEO_FORMAT_NV12,true,SPA_VIDEO_FORMAT_BGRA,false},
        {SPA_VIDEO_FORMAT_NV12,true,SPA_VIDEO_FORMAT_BGRA,true},
        {SPA_VIDEO_FORMAT_BGRA,true,SPA_VIDEO_FORMAT_NV12,false},
        {SPA_VIDEO_FORMAT_UNKNOWN,true,SPA_VIDEO_FORMAT_BGRA,true},
        {SPA_VIDEO_FORMAT_BGRA,true,SPA_VIDEO_FORMAT_NV12,true},
    };
    std::ofstream records(directory/"cases.tsv");
    require(records.good(),"cannot create parser branch records");
    records<<"index\tfirst\thas_second\tsecond\tsecond_choice\tnative_result\tnative_width\tnative_height\tnative_format\tsource_width\tsource_height\tsource_format\tsource_accepted\n";
    size_t index=0,accepted=0,unsupported=0,unknown=0;
    for(const auto& c:cases) {
        alignas(8) uint8_t bytes[4096]={};
        spa_pod_builder builder=SPA_POD_BUILDER_INIT(bytes,sizeof(bytes));
        spa_pod_frame object{},choice{};
        auto checked=[](int code){require(code>=0,"parser branch builder failed");};
        checked(spa_pod_builder_push_object(&builder,&object,SPA_TYPE_OBJECT_Format,SPA_PARAM_Format));
        checked(spa_pod_builder_add(&builder,
            SPA_FORMAT_mediaType,SPA_POD_Id(SPA_MEDIA_TYPE_video),
            SPA_FORMAT_mediaSubtype,SPA_POD_Id(SPA_MEDIA_SUBTYPE_raw),0));
        checked(spa_pod_builder_prop(&builder,SPA_FORMAT_VIDEO_format,0));
        checked(spa_pod_builder_id(&builder,c.first));
        if(c.second) {
            checked(spa_pod_builder_prop(&builder,SPA_FORMAT_VIDEO_format,0));
            if(c.choice) { checked(spa_pod_builder_push_choice(&builder,&choice,SPA_CHOICE_None,0)); }
            checked(spa_pod_builder_id(&builder,c.last));
            if(c.choice) { require(spa_pod_builder_pop(&builder,&choice)!=nullptr,"parser branch choice failed"); }
        }
        checked(spa_pod_builder_prop(&builder,SPA_FORMAT_VIDEO_size,0));
        checked(spa_pod_builder_rectangle(&builder,8,3));
        const auto* param=static_cast<const spa_pod*>(spa_pod_builder_pop(&builder,&object));
        require(param && SPA_POD_SIZE(param)<=sizeof(bytes),"parser branch POD absent/oversize");
        spa_video_info_raw native{};
        const int code=spa_format_video_raw_parse(param,&native);
        uint32_t width=0,height=0;spa_video_format format=SPA_VIDEO_FORMAT_UNKNOWN;
        MfsOriginalPipeWire::parse_video_format(param,width,height,format);
        const bool valid=width>0 && height>0 && MfsOriginalPipeWire::spa_format_info(format)!=nullptr;
        save_bytes(directory/("case-"+std::to_string(index)+".pod"),param,SPA_POD_SIZE(param));
        records<<index<<'\t'<<c.first<<'\t'<<c.second<<'\t'<<c.last<<'\t'<<c.choice<<'\t'<<code<<'\t'
            <<native.size.width<<'\t'<<native.size.height<<'\t'<<native.format<<'\t'
            <<width<<'\t'<<height<<'\t'<<static_cast<uint32_t>(format)<<'\t'<<valid<<'\n';
        // Save each native input/row before checking its branch premise. The
        // standard-parser observation distinguishes the original helper branch.
        require(code==0 && native.size.width==8 && native.size.height==3 && native.format==c.first,
            "native parser did not retain the first direct format property");
        if(c.first==SPA_VIDEO_FORMAT_NV12) {
            require(!valid && format==SPA_VIDEO_FORMAT_NV12,"original unsupported standard result fell back");
            ++unsupported;
        } else {
            require(valid && format==SPA_VIDEO_FORMAT_BGRA,"original supported/unknown branch differs");
        }
        if(c.first==SPA_VIDEO_FORMAT_UNKNOWN) { ++unknown; }
        accepted+=valid;++index;
    }
    records.close();require(records.good(),"cannot close parser branch records");
    require(index==8 && accepted==5 && unsupported==3 && unknown==2,"parser branch matrix incomplete");
    std::cout<<"original PipeWire parser branches cases=8 native_first_property=8 accepted=5 unsupported_no_fallback=3 unknown_fallback=2 capture=0 saved\n";
}
