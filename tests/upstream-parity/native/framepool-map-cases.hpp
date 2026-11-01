// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included after the shared fixture/image helpers in the standalone test TU.
int map_boundary(const std::filesystem::path& root)
{
    if (!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root)) { return 1; }
    try {
        Initialization initialization;
        winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED)); initialization.owned = true;
        std::ofstream rows(root / "map.tsv", std::ios::binary);
        rows.exceptions(std::ios::badbit | std::ios::failbit);
        rows << "cycle\tcase\tphase\toutcome\torder\tmaps\tunmaps\tconversions\tcache_clones\thresult\tsubresource\tpitch\tmapped\tviolation\tquery_order\tcache_preserved\tcache_independent\tmarker\tcaller_thread\tfixture_thread\tcontext\tresource\n";
        struct Held { std::string name; std::optional<cv::Mat> image; };
        std::vector<Held> held; held.reserve(60);
        for (uint32_t cycle = 0; cycle < 20; ++cycle) {
            const uint32_t mode = cycle % 3;
            Fixture fixture;
            if (mfs_print_thread_fixture(96, 64, 0, 0, 0, 0, MFS_RENDER_DISABLE_TRANSITIONS | MFS_RENDER_TOPMOST, &fixture.value) != 0) { throw std::runtime_error("map fixture creation failed"); }
            std::array<int64_t, 6> request { 1, WS_POPUP | WS_VISIBLE, 100, 100, 96, 64 };
            std::array<int64_t, 31> geometry {};
            if (mfs_print_thread_geometry(fixture.value.owner, request.data(), request.size(), geometry.data(), geometry.size()) != 0
                || geometry[14] != 96 || geometry[15] != 64 || mfs_desktop_fixture_inside_updates(fixture.value.owner) != 0) { throw std::runtime_error("map fixture geometry/presentation failed"); }
            std::array<uint64_t, 4> threads {};
            if (mfs_print_thread_info(fixture.value.owner, threads.data(), threads.size()) != 0 || !threads[0] || threads[0] != threads[2] || threads[0] == threads[1]) { throw std::runtime_error("map fixture owner differs"); }
            {
                MfsFrameQueryOriginal::FramePoolScreencap source(static_cast<HWND>(fixture.value.hwnd));
                // Scope ends before the original class: the observed Mat field
                // is only borrowed while its real native owner is still alive.
                mfs_map_probe::Owner observation;
                mfs_map_probe::Scope scope(observation);
                uintptr_t context = 0, resource = 0, warm_output = 0;
                for (const auto phase : { "warm", "probe", "recovery" }) {
                    const uint32_t fault = std::string_view(phase) == "probe" ? mode : 0;
                    const auto old_cache = observation.cache ? observation.cache->data : nullptr;
                    std::vector<uint8_t> old_bytes;
                    if (observation.cache) {
                        if (!observation.cache->isContinuous()) { throw std::runtime_error("source cache is not continuous"); }
                        known(*observation.cache);
                        old_bytes.assign(observation.cache->data, observation.cache->data + 18432);
                    }
                    observation.trace = {}; observation.trace.active = true; observation.trace.fault = fault;
                    trace = { true, 0, 0, 0, 0 };
                    struct Stop { ~Stop() { trace.active = false; if (mfs_map_probe::owner) { mfs_map_probe::owner->trace.active = false; } } } stop;
                    std::optional<cv::Mat> image;
                    bool conversion_exception = false;
                    try { image = source.screencap(); } // Exactly one source call.
                    catch (const mfs_map_probe::ConversionFailure&) { conversion_exception = true; }
                    trace.active = false; observation.trace.active = false;
                    const auto evidence = observation.trace;
                    const auto name = std::to_string(cycle) + "-" + phase;
                    save_pair(root, name, image, observation.cache ? std::optional<cv::Mat>(*observation.cache) : std::nullopt);
                    const uint32_t expected = fault == 1 ? 12 : fault == 2 ? 1235 : 12345;
                    if (evidence.order != expected || evidence.maps != 1 || evidence.unmaps != (fault == 1 ? 0 : 1)
                        || evidence.conversions != (fault == 1 ? 0 : 1) || evidence.cache_clones != (fault ? 0 : 1)
                        || evidence.mapped || evidence.violation || evidence.thread != threads[1]
                        || trace.order != (fault == 1 ? 0 : 123) || trace.win32 || trace.dwm
                        || (fault == 1 ? evidence.map_hresult >= 0 : evidence.map_hresult < 0)
                        || evidence.subresource != (fault == 1 ? UINT32_MAX : 0)
                        || (fault == 1 ? evidence.pitch != 0 : evidence.pitch < 384)
                        || image.has_value() != (fault == 0) || conversion_exception != (fault == 2)) { throw std::runtime_error("source Map/conversion/Unmap boundary differs"); }
                    if (!context) { context = evidence.context; resource = evidence.resource; }
                    if (!context || !resource || context != evidence.context || resource != evidence.resource
                        || !observation.cache || observation.cache->empty()) { throw std::runtime_error("source native/cache owner changed unexpectedly"); }
                    known(*observation.cache);
                    if (!observation.cache->isContinuous()) { throw std::runtime_error("source cache span is not continuous"); }
                    const bool preserved = fault != 0;
                    if (preserved && (observation.cache->data != old_cache
                        || old_bytes.size() != 18432 || memcmp(observation.cache->data, old_bytes.data(), old_bytes.size()) != 0)) { throw std::runtime_error("source error replaced warm cache"); }
                    if (image) {
                        known(*image);
                        if (!image->isContinuous() || !observation.cache->isContinuous()
                            || image->data == observation.cache->data || observation.cache->data == old_cache
                            || memcmp(image->data, observation.cache->data, 18432) != 0) { throw std::runtime_error("source cache/result ownership differs"); }
                        if (std::string_view(phase) == "warm") { warm_output = reinterpret_cast<uintptr_t>(image->data); }
                    }
                    else if (reinterpret_cast<uintptr_t>(observation.cache->data) == warm_output) { throw std::runtime_error("warm cache aliases returned image"); }
                    const int marker = image ? known(*image) : -1;
                    const uint32_t outcome = fault == 1 ? 0 : fault == 2 ? 2 : 1;
                    rows << cycle << '\t' << mode << '\t' << phase << '\t' << outcome << '\t' << evidence.order << '\t' << evidence.maps
                        << '\t' << evidence.unmaps << '\t' << evidence.conversions << '\t' << evidence.cache_clones << '\t' << evidence.map_hresult
                        << '\t' << evidence.subresource << '\t' << evidence.pitch << "\t0\t0\t" << trace.order << '\t' << preserved << "\t1\t" << marker
                        << '\t' << threads[1] << '\t' << threads[0] << '\t' << context << '\t' << resource << '\n';
                    rows.flush(); held.push_back({name, std::move(image)});
                }
            }
            fixture.close();
        }
        RoUninitialize(); initialization.owned = false;
        for (const auto& entry : held) { save(root, entry.name + "-held", entry.image); }
        std::cout << "original FramePool map_boundary cycles=20 calls=60 map_failures=7 conversion_exceptions=6 recovery=20 retained=47 passed\n";
        return 0;
    }
    catch (const winrt::hresult_error& error) { std::cerr << "map HRESULT=" << error.code().value << '\n'; }
    catch (const std::exception& error) { std::cerr << "map failure=" << error.what() << '\n'; }
    return 1;
}
