// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included inside the source oracle namespace after its owned-file/pixel helpers.
void dma_error_control() {
    MemFile file(memfd_create("mfs-invalid-dma-reference",MFD_CLOEXEC));
    std::array<uint8_t,24> input={1,2,3,4,5,6,7,8,9,0xed,0xed,0xed,11,12,13,14,15,16,17,18,19,0xed,0xed,0xed};
    require(ftruncate(file.fd,24)==0 && pwrite(file.fd,input.data(),input.size(),0)==24,"cannot prepare non-DMA error control");
    spa_chunk chunk{};chunk.stride=12;chunk.size=24;
    spa_data data{};data.type=SPA_DATA_DmaBuf;data.fd=file.fd;data.maxsize=24;data.chunk=&chunk;
    std::array<uint8_t,18> copy{};
    {
        errno=0;
        MfsOriginalPipeWire::ScopedBufferMap map(data,24);
        require(map.ptr()!=nullptr && errno==ENOTTY,"source did not preserve map after non-DMA START failure");
        require(file_mappings("mfs-invalid-dma-reference",false)==1,"missing source negative-control mapping");
        std::memcpy(copy.data(),map.ptr(),9);std::memcpy(copy.data()+9,static_cast<const uint8_t*>(map.ptr())+12,9);
    }
    require(file_mappings("mfs-invalid-dma-reference",false)==0 && fcntl(file.fd,F_GETFD)>=0,"source negative-control map/fd cleanup mismatch");
    require(std::equal(copy.begin(),copy.begin()+9,input.begin()) && std::equal(copy.begin()+9,copy.end(),input.begin()+12),"source negative-control bytes mismatch");
    data.fd=-1;data.data=input.data();errno=0;
    { MfsOriginalPipeWire::ScopedBufferMap map(data,24);require(map.ptr()==input.data() && errno==0,"source existing pointer performed native sync/mapping"); }
    require(errno==0,"source existing-pointer destructor performed cleanup");
    std::cout<<"original non-DMA error control START=ENOTTY borrowed_pointer=skips_sync retained_bytes=1 passed; not DMA acceptance\n";
}
int save_dma_fixtures(const std::filesystem::path& directory) {
    std::filesystem::create_directories(directory);
    { std::ofstream abi(directory/"abi.txt");
      for(uint32_t i=0;i<7;++i) { abi<<mfs_pw_dma_abi(i)<<'\n'; }
      abi.close();require(abi.good(),"cannot save DMA ioctl ABI"); }
    uint32_t exporter=0;
    std::ofstream availability(directory/"availability.txt");
    for(uint32_t candidate:{1u,2u}) {
        const int fd=mfs_pw_dma_create(candidate,4096);
        availability<<"exporter="<<candidate<<" fd_open="<<(fd>=0)<<" errno="<<(fd<0 ? -fd : 0)<<'\n';
        if(fd>=0) { close(fd);exporter=candidate;break; }
        require(fd==-ENOENT || fd==-ENODEV || fd==-EACCES || fd==-EPERM,
            "DMA exporter failed with an unexpected native error");
    }
    availability.close();require(availability.good(),"cannot save DMA exporter observations");
    { std::ofstream selected(directory/"exporter.txt");selected<<exporter;selected.close();require(selected.good(),"cannot save DMA exporter"); }
    if(!exporter) {
        std::cout<<"missing_environment: no accessible udmabuf or system DMA heap exporter; no DMA capture ran\n";
        return 77;
    }
    cv::Mat warm(HEIGHT,WIDTH,CV_8UC3,cv::Scalar(1,2,3)),warm_bgr;
    cv::cvtColor(warm,warm_bgr,cv::COLOR_RGB2BGR);
    std::vector<HeldImage> held;held.reserve(60);
    constexpr std::array<std::array<uint8_t,3>,7> bars={{{191,191,191},{191,191,0},{0,191,191},{0,191,0},{191,0,191},{191,0,0},{0,0,191}}};
    std::ofstream cases(directory/"cases.tsv");
    cases<<"index\tstride\tmaxsize\thas_chunk\tmarker\n";
    const auto baseline=count_entries("/proc/self/fd");
    for(int cycle=0;cycle<20;++cycle) {
        MemFile dma(mfs_pw_dma_create(exporter,4096));
        const int flags=fcntl(dma.fd,F_GETFD);
        require(flags>=0 && (flags&FD_CLOEXEC)!=0,"exported DMA fd lacks CLOEXEC");
        struct stat info{};require(fstat(dma.fd,&info)==0 && info.st_size==4096,"DMA exporter size evidence mismatch");
        if(cycle==0) {
            std::ofstream evidence(directory/"fdinfo.txt");
            const auto read=write_task_file(evidence,std::filesystem::path("/proc/self/fdinfo")/std::to_string(dma.fd));
            evidence.close();require(evidence.good() && read.close_error==0,"cannot save real DMA fdinfo");
        }
        for(int mode=0;mode<3;++mode) {
            const int index=cycle*3+mode,stride=mode==2 ? WIDTH*3 : 96;
            const uint32_t marker=static_cast<uint32_t>(index+1);
            std::vector<uint8_t> bytes(static_cast<size_t>(stride)*HEIGHT,0xed);
            for(int y=0;y<HEIGHT;++y) { for(int x=0;x<WIDTH;++x) { for(int c=0;c<3;++c) {
                bytes[static_cast<size_t>(y)*stride+x*3+c]=static_cast<uint8_t>(bars[static_cast<size_t>(x)/4][static_cast<size_t>(c)]^((y*7+x*3+c*13+(marker&63))&63));
            } } }
            bytes[0]=static_cast<uint8_t>(marker>>16);bytes[1]=static_cast<uint8_t>(marker>>8);bytes[2]=static_cast<uint8_t>(marker);
            require(mfs_pw_dma_write(dma.fd,bytes.data(),bytes.size())==0,"real DMA producer WRITE sync/copy failed");
            spa_chunk chunk{};chunk.stride=stride;chunk.size=static_cast<uint32_t>(bytes.size());
            spa_data data{};data.type=SPA_DATA_DmaBuf;data.flags=SPA_DATA_FLAG_READABLE;data.fd=dma.fd;
            data.maxsize=mode==1 ? 0 : static_cast<uint32_t>(bytes.size());data.chunk=mode==2 ? nullptr : &chunk;
            cv::Mat raw(HEIGHT,WIDTH,CV_8UC3);
            { MfsOriginalPipeWire::ScopedBufferMap mapping(data,bytes.size());
              require(mapping.ptr(),"original real DMA mapping failed");
              for(int y=0;y<HEIGHT;++y) { std::memcpy(raw.ptr(y),static_cast<const uint8_t*>(mapping.ptr())+static_cast<size_t>(y)*stride,WIDTH*3); } }
            require(fcntl(dma.fd,F_GETFD)>=0,"original map consumed DMA fd");
            cv::Mat bgr;cv::cvtColor(raw,bgr,cv::COLOR_RGB2BGR);
            require(known_pixels(bgr)==marker,"original DMA helper pixels mismatch");
            save(raw,directory/("case-"+std::to_string(index)+".rgb"));
            save(bgr,directory/("case-"+std::to_string(index)+".bgr"));
            { std::ofstream input(directory/("case-"+std::to_string(index)+".input"),std::ios::binary);
              input.write(reinterpret_cast<const char*>(bytes.data()),static_cast<std::streamsize>(bytes.size()));input.close();require(input.good(),"cannot save DMA input"); }
            cases<<index<<'\t'<<stride<<'\t'<<data.maxsize<<'\t'<<(mode!=2)<<'\t'<<marker<<'\n';
            held.push_back({std::move(bgr),marker});
            std::fill(bytes.begin(),bytes.end(),0);
            require(mfs_pw_dma_write(dma.fd,bytes.data(),bytes.size())==0,"cannot overwrite released DMA input");
            for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"DMA native overwrite changed held source pixels"); }
        }
        dma.close();require(count_entries("/proc/self/fd")==baseline,"source DMA helper fd leak");
    }
    cases.close();require(cases.good(),"cannot save DMA cases");
    for(const auto& image:held) { require(known_pixels(image.image)==image.marker,"closed DMA source frame changed"); }
    std::cout<<"original DMA helper exporter="<<exporter<<" cycles=20 maps=60 retained_bgr=60 fd_preserved=60 passed; incoming stream/GPU completion pending\n";
    return 0;
}
