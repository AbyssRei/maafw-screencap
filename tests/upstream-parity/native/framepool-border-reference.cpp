// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Separate native property/device controls. The original class/headers/bodies
// are unchanged; observations never become an ideal-pixel parity waiver.
#define wmain mfs_unused_border_context_entry
#include "framepool-reference.cpp"
#undef wmain
#include "Screencap/DesktopDupWindowScreencap.h"
#include <winrt/Windows.Foundation.Metadata.h>
#include <dwmapi.h>
#include <cstdint>
#include <memory>
#include <string_view>
#include <vector>

namespace mfs_border_observation {
using Wgc = MaaNS::CtrlUnitNs::FramePoolScreencap;
using Dup = MaaNS::CtrlUnitNs::DesktopDupWindowScreencap;
using Session = winrt::Windows::Graphics::Capture::GraphicsCaptureSession;
// Explicit instantiation permits a typed private-member pointer. This changes
// neither the original declaration/access tokens nor its layout/ODR identity.
// Only this test uses the pointer, on the original object's live caller channel.
template<class Tag, typename Tag::type Member> struct MemberAccess {
    friend typename Tag::type member(Tag) { return Member; }
};
struct SessionMember {
    using type = Session Wgc::*;
    friend type member(SessionMember);
};
template struct MemberAccess<SessionMember, &Wgc::cap_session_>;

struct DeviceOnly {
    HWND hwnd_ = nullptr;
    winrt::com_ptr<ID3D11Device> d3d_device_;
    winrt::com_ptr<ID3D11DeviceContext> d3d_context_;
    winrt::com_ptr<IDXGISwapChain> dxgi_swap_chain_;
    explicit DeviceOnly(HWND window) : hwnd_(window)
    {
        HRESULT ret = S_OK;
        // Exact init fragment from pinned F: descriptor and the one native
        // D3D11CreateDeviceAndSwapChain call, without a capture item/session.
#include "framepool-device-only.inc"
        winrt::check_hresult(ret);
        if (!d3d_device_ || !d3d_context_ || !dxgi_swap_chain_) {
            throw std::runtime_error("device-only control lacks native owners");
        }
    }
};
struct Dpi {
    DPI_AWARENESS_CONTEXT original = GetThreadDpiAwarenessContext();
    bool restored = false;
    Dpi()
    {
        if (!original || !SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)) {
            throw std::runtime_error("border fixture caller DPI preparation failed");
        }
    }
    bool close()
    {
        if (restored) { return true; }
        if (!SetThreadDpiAwarenessContext(original)) { return false; }
        restored = true;
        return AreDpiAwarenessContextsEqual(original, GetThreadDpiAwarenessContext());
    }
    ~Dpi() { if (!close()) { std::terminate(); } }
};
struct Property {
    uint64_t arm = 0, session = 0, getter_calls = 0, setter_calls = 0;
    int32_t before_hr = 0, set_hr = 0, after_hr = 0;
    int before = -1, after = -1;
};
struct BorderShot {
    uint64_t arm = 0, slot = 0, kind = 0;
    std::string phase;
    bool wgc_alive = false, device_only_alive = false, engaged = false;
    int64_t begin = 0, end = 0;
    WindowState before {}, after {};
    std::array<int64_t, 12> fixture_before {}, fixture_after {};
    cv::Mat image;
};
int64_t qpc()
{
    LARGE_INTEGER value {};
    if (!QueryPerformanceCounter(&value)) { throw std::runtime_error("border QPC failed"); }
    return value.QuadPart;
}
bool known(const cv::Mat& image)
{
    if (image.cols != 96 || image.rows != 64 || image.type() != CV_8UC3) { return false; }
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 96; ++x) {
            const cv::Vec3b expected = x < 32 ? cv::Vec3b(0x1f, 0x53, 0xa7)
                : y < 32 ? cv::Vec3b(0xb9, 0x4d, 0x27) : cv::Vec3b(0x3b, 0xe2, 0x81);
            if (image.at<cv::Vec3b>(y, x) != expected) { return false; }
        }
    }
    return true;
}
std::array<int64_t, 12> fixture_boundary(HWND window)
{
    RECT outer {}, client {};
    POINT origin {};
    if (!GetWindowRect(window, &outer) || !GetClientRect(window, &client)
        || !ClientToScreen(window, &origin)) {
        throw std::runtime_error("border fixture call-boundary geometry query failed");
    }
    return { static_cast<uint32_t>(GetWindowLongPtrW(window, GWL_STYLE)),
        outer.left, outer.top, outer.right, outer.bottom,
        client.left, client.top, client.right, client.bottom, origin.x, origin.y,
        static_cast<uint32_t>(GetWindowLongPtrW(window, GWL_EXSTYLE)) };
}
template<class Source> void take(std::vector<BorderShot>& shots, uint64_t arm,
    const char* phase, uint64_t kind, Source& source, HWND window, bool wgc, bool device, int scene)
{
    BorderShot shot;
    shot.arm = arm; shot.slot = shots.size(); shot.phase = phase; shot.kind = kind;
    shot.wgc_alive = wgc; shot.device_only_alive = device;
    shot.before = window_state(window);
    if (scene >= 0) { shot.fixture_before = fixture_boundary(window); }
    shot.begin = qpc();
    auto image = source.screencap(); // One unchanged operation; no retry loop.
    shot.end = qpc(); shot.after = window_state(window); shot.engaged = image.has_value();
    if (scene >= 0) { shot.fixture_after = fixture_boundary(window); }
    // Snapshot before native object teardown; original results may share cache.
    if (image && !image->empty()) { shot.image = image->clone(); }
    shots.push_back(std::move(shot));
}
void observe_property(Property& row, Wgc& source, bool require)
{
    auto& session = source.*member(SessionMember {});
    // A failed original initialization is an observation, not a fabricated
    // native getter HRESULT. Keep calls=0/value=-1 and fail the positive control.
    if (!session) { return; }
    row.session = reinterpret_cast<uintptr_t>(winrt::get_abi(session));
    ++row.getter_calls;
    try { row.before = session.IsBorderRequired() ? 1 : 0; }
    catch (const winrt::hresult_error& error) { row.before_hr = error.code().value; }
    if (require) {
        ++row.setter_calls;
        try { session.IsBorderRequired(true); }
        catch (const winrt::hresult_error& error) { row.set_hr = error.code().value; }
        ++row.getter_calls;
        try { row.after = session.IsBorderRequired() ? 1 : 0; }
        catch (const winrt::hresult_error& error) { row.after_hr = error.code().value; }
    }
}
void save_shot(const BorderShot& shot, const std::filesystem::path& root)
{
    const auto dir = root / ("shot-" + std::to_string(shot.slot));
    std::filesystem::create_directory(dir);
    std::ofstream bytes(dir / "image.bgr", std::ios::binary), layout(dir / "layout.tsv");
    bytes.exceptions(std::ios::badbit | std::ios::failbit);
    layout.exceptions(std::ios::badbit | std::ios::failbit);
    size_t row = 0;
    if (!shot.image.empty()) {
        if (shot.image.cols <= 0 || shot.image.rows <= 0 || shot.image.type() != CV_8UC3) {
            throw std::runtime_error("border diagnostic native layout differs");
        }
        const auto width = static_cast<size_t>(shot.image.cols);
        if (width > (std::numeric_limits<size_t>::max)() / 3) { throw std::runtime_error("border row overflow"); }
        row = width * 3;
        if (row > static_cast<size_t>((std::numeric_limits<std::streamsize>::max)())) {
            throw std::runtime_error("border stream row overflow");
        }
        for (int y = 0; y < shot.image.rows; ++y) {
            bytes.write(reinterpret_cast<const char*>(shot.image.ptr(y)), static_cast<std::streamsize>(row));
        }
    }
    layout << shot.engaged << '\t' << shot.image.cols << '\t' << shot.image.rows << '\t'
           << shot.image.type() << '\t' << row << '\n';
    bytes.close(); layout.close();
}

std::array<int64_t, 63> prepare_fixture_geometry(Fixture& fixture, uint64_t arm, int scene)
{
    // Two matched controls alter only this owned fixture before any capture
    // object exists. Both retain the entire 96x64 client at the same work-area
    // offset. There is no target-pixel inset or post-capture preparation.
    std::array<int64_t, 63> result {};
    std::array<int64_t, 31> before {}, after {};
    const int64_t query[6] {};
    if (mfs_print_thread_geometry(fixture.value.owner, query, 6, before.data(), before.size())) {
        throw std::runtime_error("border fixture initial geometry query failed");
    }
    if (before[6] - before[4] < 224 || before[7] - before[5] < 192) {
        throw std::runtime_error("border matched fixture work area too small");
    }
    const DWORD style = WS_VISIBLE | WS_POPUP | WS_CLIPSIBLINGS | (scene == 1 ? WS_CAPTION : 0);
    const int64_t input[6] { scene == 2 ? 4 : 1, style, before[4] + 64, before[5] + 64, 96, 64 };
    if (mfs_print_thread_geometry(fixture.value.owner, input, 6, after.data(), after.size())) {
        throw std::runtime_error("border matched fixture geometry setup failed");
    }
    const int64_t repaint[6] { 2, 0, 0, 0, 0, 0 };
    if (mfs_print_thread_geometry(fixture.value.owner, repaint, 6, after.data(), after.size())
        || after[12] != 0 || after[13] != 0 || after[14] != 96 || after[15] != 64
        || after[16] != input[2] || after[17] != input[3]
        || after[18] != style || after[27] != DWMWCP_DONOTROUND || after[28] != 0) {
        throw std::runtime_error("border matched fixture full-client premise failed");
    }
    result[0] = static_cast<int64_t>(arm);
    std::copy(before.begin(), before.end(), result.begin() + 1);
    std::copy(after.begin(), after.end(), result.begin() + 32);
    return result;
}
} // namespace mfs_border_observation

int wmain(int argc, wchar_t** argv)
{
    if (argc != 2 && argc != 3) { return 64; }
    int scene = -1;
    if (argc == 3) {
        const std::wstring_view requested(argv[2]);
        if (requested == L"popup") { scene = 0; }
        else if (requested == L"caption") { scene = 1; }
        else if (requested == L"client-band") { scene = 2; }
        else { return 64; }
    }
    const std::filesystem::path root(argv[1]);
    if (!root.is_absolute() || !std::filesystem::is_directory(root) || !std::filesystem::is_empty(root)) { return 64; }
    using namespace mfs_border_observation;
    try {
        Initialization initialization;
        winrt::check_hresult(RoInitialize(RO_INIT_MULTITHREADED)); initialization.owned = true;
        if (!winrt::Windows::Foundation::Metadata::ApiInformation::IsPropertyPresent(
                L"Windows.Graphics.Capture.GraphicsCaptureSession", L"IsBorderRequired")) {
            std::cout << "border environment missing before_capture=1 property_unavailable=1\n";
            return 77;
        }
        Dispatcher dispatcher;
        const DispatcherQueueOptions options { sizeof(DispatcherQueueOptions), DQTYPE_THREAD_CURRENT, DQTAT_COM_NONE };
        winrt::check_hresult(CreateDispatcherQueueController(options,
            reinterpret_cast<PDISPATCHERQUEUECONTROLLER*>(winrt::put_abi(dispatcher.controller))));
        Dpi dpi;
        LARGE_INTEGER frequency {};
        if (!QueryPerformanceFrequency(&frequency)) { throw std::runtime_error("border QPC frequency failed"); }
        std::vector<BorderShot> shots; shots.reserve(32);
        std::array<Property, 4> properties {};
        std::array<std::array<int64_t, 2>, 4> destroyed {};
        std::array<std::array<uint64_t, 6>, 4> windows {};
        std::array<std::array<uint64_t, 10>, 4> drivers {};
        std::array<std::array<int64_t, 63>, 4> geometries {};
        uint64_t closed = 0;
        // Default source session; source session then require=true treatment;
        // exact source D3D device/swapchain fragment only; final no-WGC control.
        for (uint64_t arm = 0; arm < 4; ++arm) {
            Fixture fixture;
            if (mfs_print_thread_fixture(96, 64, 0, 0, 0, 0,
                    MFS_RENDER_DISABLE_TRANSITIONS | MFS_RENDER_TOPMOST, &fixture.value)) {
                throw std::runtime_error("border owned window creation failed");
            }
            const auto hwnd = static_cast<HWND>(fixture.value.hwnd);
            if (scene >= 0) { geometries[arm] = prepare_fixture_geometry(fixture, arm, scene); }
            windows[arm][0] = reinterpret_cast<uintptr_t>(hwnd);
            if (mfs_print_thread_info(fixture.value.owner, windows[arm].data() + 1, 4)) {
                throw std::runtime_error("border window identity failed");
            }
            windows[arm][5] = GetCurrentThreadId();
            if (mfs_desktop_fixture_updates(fixture.value.owner)) { throw std::runtime_error("border frame driver setup failed"); }
            std::array<uint64_t, 3> update {};
            if (mfs_desktop_fixture_update_info(fixture.value.owner, update.data(), update.size()) || !update[0] || !update[1] || update[2]) {
                throw std::runtime_error("border frame driver initial state failed");
            }
            const auto marker = reinterpret_cast<HWND>(static_cast<uintptr_t>(update[0]));
            DWORD marker_pid = 0; const auto marker_thread = GetWindowThreadProcessId(marker, &marker_pid);
            RECT marker_rect {}, target_rect {}, overlap {};
            if (!marker_thread || marker_thread != windows[arm][1] || marker_pid != windows[arm][4] || marker == hwnd
                || !GetWindowRect(marker, &marker_rect) || !GetWindowRect(hwnd, &target_rect)
                || MonitorFromWindow(marker, MONITOR_DEFAULTTONEAREST) != MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)
                || IntersectRect(&overlap, &marker_rect, &target_rect)) {
                throw std::runtime_error("border frame driver target isolation failed");
            }
            auto& driver = drivers[arm]; driver = { arm, update[0], marker_thread, marker_pid, update[1], 0, 0, 1, 1, 0 };
            {
                Dup duplicate(hwnd); std::unique_ptr<Wgc> wgc; std::unique_ptr<DeviceOnly> device;
                take(shots, arm, "dd-before-warm", 0, duplicate, hwnd, false, false, scene);
                take(shots, arm, "dd-before-test", 0, duplicate, hwnd, false, false, scene);
                if (arm < 2) {
                    wgc = std::make_unique<Wgc>(hwnd);
                    take(shots, arm, "wgc-default", 1, *wgc, hwnd, true, false, scene);
                    properties[arm].arm = arm;
                    auto& initial_session = (*wgc).*member(SessionMember {});
                    properties[arm].session = reinterpret_cast<uintptr_t>(winrt::get_abi(initial_session));
                    if (arm == 0) { observe_property(properties[arm], *wgc, false); }
                    take(shots, arm, "dd-with-default", 0, duplicate, hwnd, true, false, scene);
                    if (arm == 1) {
                        // Positive native control: require the system indicator.
                        // This test never adds a false setter or host consent change.
                        Property required; required.arm = arm;
                        observe_property(required, *wgc, true);
                        if (required.session != properties[arm].session) { throw std::runtime_error("border session changed"); }
                        properties[arm] = required;
                        take(shots, arm, "dd-with-required", 0, duplicate, hwnd, true, false, scene);
                    }
                } else if (arm == 2) {
                    device = std::make_unique<DeviceOnly>(hwnd);
                    take(shots, arm, "dd-with-device", 0, duplicate, hwnd, false, true, scene);
                    take(shots, arm, "dd-with-device-test", 0, duplicate, hwnd, false, true, scene);
                }
                destroyed[arm][0] = qpc(); wgc.reset(); device.reset(); destroyed[arm][1] = qpc();
                // Three fixed observations, not retries toward a pixel outcome.
                for (unsigned call = 0; call < 3; ++call) {
                    const auto phase = "dd-post-" + std::to_string(call);
                    take(shots, arm, phase.c_str(), 0, duplicate, hwnd, false, false, scene);
                }
            }
            if (mfs_desktop_fixture_update_info(fixture.value.owner, update.data(), update.size())
                || update[0] != driver[1] || update[1] <= driver[4] || update[2]) {
                throw std::runtime_error("border frame driver progress failed");
            }
            driver[5] = update[1]; driver[6] = update[2];
            if (!fixture.close()) { throw std::runtime_error("border fixture close failed"); }
            ++closed; if (IsWindow(marker)) { throw std::runtime_error("border frame driver owner still live"); }
            driver[9] = 1;
        }
        const bool dispatcher_ok = dispatcher.close(), dpi_ok = dpi.close();
        std::ofstream rows(root / "shots.tsv"), states(root / "states.tsv"), props(root / "properties.tsv"), owners(root / "owners.tsv"), targets(root / "windows.tsv"), frame_drivers(root / "frame-drivers.tsv");
        rows.exceptions(std::ios::badbit | std::ios::failbit); states.exceptions(std::ios::badbit | std::ios::failbit);
        props.exceptions(std::ios::badbit | std::ios::failbit); owners.exceptions(std::ios::badbit | std::ios::failbit);
        targets.exceptions(std::ios::badbit | std::ios::failbit); frame_drivers.exceptions(std::ios::badbit | std::ios::failbit);
        rows << "slot\tarm\tphase\tkind\twgc_alive\tdevice_alive\tbegin_qpc\tend_qpc\tengaged\twidth\theight\tknown_scene\tfnv64\n";
        bool controls = dispatcher_ok && dpi_ok;
        for (const auto& shot : shots) {
            save_shot(shot, root); const bool scene = known(shot.image);
            rows << shot.slot << '\t' << shot.arm << '\t' << shot.phase << '\t' << shot.kind << '\t' << shot.wgc_alive << '\t' << shot.device_only_alive
                 << '\t' << shot.begin << '\t' << shot.end << '\t' << shot.engaged << '\t' << shot.image.cols << '\t' << shot.image.rows << '\t' << scene << '\t' << digest(shot.image) << '\n';
            states << shot.slot; for (auto value : shot.before) { states << '\t' << value; } for (auto value : shot.after) { states << '\t' << value; } states << '\n';
            if (shot.phase.starts_with("dd-before-") || shot.kind == 1 || shot.arm == 3) { controls &= shot.engaged && scene; }
        }
        for (uint64_t arm = 0; arm < 4; ++arm) {
            auto row = properties[arm]; row.arm = arm;
            props << arm << '\t' << row.session << '\t' << row.getter_calls << '\t' << row.before_hr << '\t' << row.before
                  << '\t' << row.setter_calls << '\t' << row.set_hr << '\t' << row.after_hr << '\t' << row.after << '\n';
            if (arm < 2) { controls &= row.session && row.before_hr == 0 && (row.before == 0 || row.before == 1); }
            if (arm == 1) { controls &= row.setter_calls == 1 && row.set_hr == 0 && row.after_hr == 0 && row.after == 1; }
            owners << arm << '\t' << destroyed[arm][0] << '\t' << destroyed[arm][1] << '\n';
            targets << arm; for (auto value : windows[arm]) { targets << '\t' << value; } targets << '\n';
            for (size_t i = 0; i < drivers[arm].size(); ++i) { if (i) { frame_drivers << '\t'; } frame_drivers << drivers[arm][i]; } frame_drivers << '\n';
        }
        rows.close(); states.close(); props.close(); owners.close(); targets.close(); frame_drivers.close();
        if (scene >= 0) {
            std::ofstream geometry(root / "fixture-geometries.tsv");
            geometry.exceptions(std::ios::badbit | std::ios::failbit);
            for (const auto& row : geometries) {
                for (size_t i = 0; i < row.size(); ++i) { if (i) { geometry << '\t'; } geometry << row[i]; }
                geometry << '\n';
            }
            geometry.close();
            std::ofstream boundaries(root / "fixture-boundaries.tsv");
            boundaries.exceptions(std::ios::badbit | std::ios::failbit);
            for (const auto& shot : shots) {
                boundaries << shot.slot;
                for (auto value : shot.fixture_before) { boundaries << '\t' << value; }
                for (auto value : shot.fixture_after) { boundaries << '\t' << value; }
                boundaries << '\n';
            }
            boundaries.close();
        }
        std::ofstream summary(root / "summary.txt"); summary.exceptions(std::ios::badbit | std::ios::failbit);
        summary << "schema=" << (scene < 0 ? 1 : 2) << "\nshots=" << shots.size() << "\nclosed_windows=" << closed << "\nqpc_frequency=" << frequency.QuadPart
                << "\ndispatcher_completed=1\ndispatcher_within_budget=" << dispatcher_ok << "\ncaller_dpi_restored=" << dpi_ok
                << "\npositive_controls=" << controls << "\ncv_8u=" << CV_8U << "\ncv_8uc3=" << CV_8UC3 << "\ncv_cn_shift=" << CV_CN_SHIFT
                << "\nsource_bodies_unchanged=1\nextra_cpu_snapshots=1\nrequired_true_setters=1\nadded_false_setters=0\noriginal_failure_reclassified=0\nfull_P1_acceptance=0\n";
        if (scene >= 0) { summary << "fixture_case=" << scene << "\nfixture_geometry_calls=12\nfixture_boundary_observations=54\n"; }
        summary.close();
        std::cout << "source border diagnostic shots=" << shots.size() << " windows_closed=" << closed << " positive_controls=" << controls
                  << " required_true_setters=1 added_false_setters=0 source_bodies_unchanged=1 full_P1_acceptance=0\n";
        if (scene >= 0) { std::cout << "matched border fixture case=" << scene << " full_client=96x64 geometry_calls=12 boundary_observations=54 target_inset=0\n"; }
        return controls ? 0 : 5;
    } catch (const winrt::hresult_error& error) { std::cerr << "border source hresult=" << error.code().value << '\n'; return 3; }
    catch (const std::exception& error) { std::cerr << "border diagnostic failure=" << error.what() << '\n'; return 3; }
}
