// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
int size_boundary(const std::filesystem::path& root)
{
    try {
        std::filesystem::create_directories(root);
        Initialization initialization; winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED)); initialization.owned = true;
        std::ofstream rows(root / "size.tsv", std::ios::binary), queries(root / "queries.tsv", std::ios::binary);
        rows.exceptions(std::ios::badbit | std::ios::failbit); queries.exceptions(std::ios::badbit | std::ios::failbit);
        rows << "cycle\tcase\tphase\toutcome\terror\tsize_calls\tbefore_slots\tafter_slots\tlast_width\tlast_height\tcache_preserved\tcache_independent\tmarker\tcaller_thread\tfixture_thread\n";
        queries << "cycle\tphase\tordinal\tnative_hresult\tnative_width\tnative_height\treturned_hresult\treturned_width\treturned_height\tcaller_thread\titem\n";
        struct Held { std::string name; std::optional<cv::Mat> image; };
        std::vector<Held> held; held.reserve(60);
        constexpr std::array<uint32_t, 10> after { 127, 23, 7, 119, 7, 23, 119, 23, 7, 7 };
        for (uint32_t cycle = 0; cycle < 20; ++cycle) {
            const auto mode = cycle % 10;
            Fixture fixture;
            if (mfs_print_thread_fixture(96, 64, 0, 0, 0, 0, MFS_RENDER_DISABLE_TRANSITIONS | MFS_RENDER_TOPMOST, &fixture.value) != 0) { throw std::runtime_error("Size fixture create failed"); }
            std::array<int64_t, 6> request { 1, WS_POPUP | WS_VISIBLE, 100, 100, 96, 64 };
            std::array<int64_t, 31> geometry {}; std::array<uint64_t, 4> threads {};
            if (mfs_print_thread_geometry(fixture.value.owner, request.data(), request.size(), geometry.data(), geometry.size()) != 0
                || geometry[14] != 96 || geometry[15] != 64 || mfs_desktop_fixture_inside_updates(fixture.value.owner) != 0
                || mfs_print_thread_info(fixture.value.owner, threads.data(), threads.size()) != 0 || !threads[0] || threads[0] == threads[1]) { throw std::runtime_error("Size fixture geometry/owner failed"); }
            {
                MfsFrameQueryOriginal::FramePoolScreencap source(static_cast<HWND>(fixture.value.hwnd));
                for (const auto* phase : { "warm", "probe", "recovery" }) {
                    const bool probe = std::string_view(phase) == "probe";
                    if (probe && ((mode >= 1 && mode <= 3) || mode >= 8)) { source.uninit(); }
                    const auto before = mfs_size_probe::slots(source);
                    const auto* old_address = source.cached_image_.data;
                    auto old = source.cached_image_.clone();
                    mfs_size_probe::Trace trace {}; trace.mode = probe ? mode : 0;
                    std::optional<cv::Mat> image; int32_t error = 0; uint32_t outcome = 0;
                    { mfs_size_probe::Scope scope(trace);
                      try { image = source.screencap(); outcome = image ? 1 : 0; }
                      catch (const winrt::hresult_error& failure) { error = failure.code(); outcome = 2; } }
                    const auto name = std::to_string(cycle) + "-" + phase;
                    save(root, name, image); save(root, name + "-cache", std::optional<cv::Mat>(source.cached_image_));
                    const auto expected_outcome = !probe || mode == 0 ? 1u : mode == 1 || mode == 3 || mode == 6 || mode == 7 ? 2u : 0u;
                    if (outcome != expected_outcome || (outcome == 2 ? error != mfs_size_probe::injected : error != 0)
                        || mfs_size_probe::slots(source) != (probe ? after[mode] : 127u)
                        || source.last_capture_size_ != (probe && mode != 0 ? std::pair(0, 0) : std::pair(96, 64))) { throw std::runtime_error("Size catch/partial state differs"); }
                    const bool preserved = probe && mode != 0;
                    if (preserved && (source.cached_image_.data != old_address || old.total() != 6144 || memcmp(old.data, source.cached_image_.data, 18432))) { throw std::runtime_error("Size failure replaced warm cache"); }
                    known(source.cached_image_);
                    if (image && (image->data == source.cached_image_.data || memcmp(image->data, source.cached_image_.data, 18432))) { throw std::runtime_error("Size output/cache aliases or differs"); }
                    const int marker = image ? known(*image) : -1;
                    rows << cycle << '\t' << mode << '\t' << phase << '\t' << outcome << '\t' << error << '\t' << trace.calls
                        << '\t' << before << '\t' << mfs_size_probe::slots(source) << '\t' << source.last_capture_size_.first << '\t' << source.last_capture_size_.second
                        << '\t' << preserved << "\t1\t" << marker << '\t' << threads[1] << '\t' << threads[0] << '\n';
                    for (uint32_t i = 0; i < trace.calls; ++i) {
                        const auto& q = trace.queries[i];
                        if (q.native_code || q.width != 96 || q.height != 64 || q.thread != threads[1] || !q.item) { throw std::runtime_error("real Size query/channel differs"); }
                        queries << cycle << '\t' << phase << '\t' << i+1 << '\t' << q.native_code << '\t' << q.width << '\t' << q.height << '\t' << q.returned_code << '\t' << q.returned_width << '\t' << q.returned_height << '\t' << q.thread << '\t' << q.item << '\n';
                    }
                    rows.flush(); queries.flush(); held.push_back({name, std::move(image)});
                }
            }
            fixture.close();
        }
        RoUninitialize(); initialization.owned = false;
        for (const auto& value : held) { save(root, value.name + "-held", value.image); }
        std::cout << "original FramePool size_boundary cycles=20 calls=60 rejected=18 recovery=20 retained=42 passed\n"; return 0;
    }
    catch (const winrt::hresult_error& error) { std::cerr << "Size native HRESULT=" << error.code().value << '\n'; }
    catch (const std::exception& error) { std::cerr << "Size failure=" << error.what() << '\n'; }
    return 1;
}
