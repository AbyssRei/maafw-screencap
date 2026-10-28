// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Isolated query interposition over the exact original function bodies.
#include "print-query-probe.h"
#include "gdi-probe-internal.hpp"
#include "Base/UnitBase.h"
#include "Common/Conf.h"
#include "MaaUtils/Logger.h"
#include <algorithm>
#include <array>
#include <new>

namespace {
struct QueryProbe {
    bool active = false;
    HWND main = nullptr;
    uint32_t scenario = 0;
    MfsPrintQueryReport report {};
};
thread_local QueryProbe query;
void observed(uint32_t operation, HWND hwnd, bool result)
{
    if (!query.active) { return; }
    if (query.report.count == 16) { query.report.overflow = 1; return; }
    query.report.events[query.report.count++] = {
        operation, hwnd != query.main ? 1u : 0u, result ? 1 : 0, GetLastError()
    };
}
HWND query_root(HWND hwnd, UINT flags)
{
    SetLastError(0);
    const HWND result = query.scenario == 1 ? nullptr : GetAncestor(hwnd, flags);
    observed(1, hwnd, result != nullptr);
    return result;
}
HWND query_popup(HWND hwnd)
{
    SetLastError(0);
    const HWND result = query.scenario == 2 ? nullptr
        : query.scenario == 3 ? hwnd : GetLastActivePopup(hwnd);
    observed(2, hwnd, result != nullptr);
    return result;
}
BOOL query_valid(HWND hwnd)
{
    SetLastError(0);
    const BOOL result = query.scenario == 4 && hwnd != query.main ? FALSE : IsWindow(hwnd);
    observed(3, hwnd, result != FALSE);
    return result;
}
BOOL query_visible(HWND hwnd)
{
    SetLastError(0);
    const BOOL result = query.scenario == 5 ? FALSE : IsWindowVisible(hwnd);
    observed(4, hwnd, result != FALSE);
    return result;
}
BOOL query_origin(HWND hwnd, POINT* point)
{
    SetLastError(0);
    BOOL result = FALSE;
    if ((query.scenario == 6 && hwnd == query.main) || (query.scenario == 7 && hwnd != query.main)) {
        SetLastError(0xe0f10000u | query.scenario);
    }
    else { result = ClientToScreen(hwnd, point); }
    observed(5, hwnd, result != FALSE);
    return result;
}
}

int32_t mfs_print_query_begin(void* main, uint32_t scenario)
{
    if (query.active || !main || scenario > 7) { return 1; }
    query = {};
    query.active = true;
    query.main = static_cast<HWND>(main);
    query.scenario = scenario;
    return 0;
}
int32_t mfs_print_query_end(MfsPrintQueryReport* output)
{
    if (!query.active || !output) { return 1; }
    *output = query.report;
    query = {};
    return 0;
}
extern "C" {
static void* api_root(void* hwnd) { return query_root(static_cast<HWND>(hwnd), GA_ROOTOWNER); }
static void* api_popup(void* hwnd) { return query_popup(static_cast<HWND>(hwnd)); }
static int32_t api_valid(void* hwnd) { return query_valid(static_cast<HWND>(hwnd)); }
static int32_t api_visible(void* hwnd) { return query_visible(static_cast<HWND>(hwnd)); }
static int32_t api_origin(void* hwnd, void* point) { return query_origin(static_cast<HWND>(hwnd), static_cast<POINT*>(point)); }
}
int32_t mfs_print_query_api(MfsPrintQueryApi* output)
{
    if (!output) { return 1; }
    *output = { api_root, api_popup, api_valid, api_visible, api_origin };
    return 0;
}
int32_t mfs_print_query_layout(uint64_t* output, size_t capacity)
{
    constexpr std::array<uint64_t, 12> layout {
        sizeof(MfsPrintQueryEvent), alignof(MfsPrintQueryEvent), offsetof(MfsPrintQueryEvent, popup),
        offsetof(MfsPrintQueryEvent, result), offsetof(MfsPrintQueryEvent, last_error),
        sizeof(MfsPrintQueryReport), alignof(MfsPrintQueryReport), offsetof(MfsPrintQueryReport, events),
        sizeof(MfsPrintQueryApi), alignof(MfsPrintQueryApi), offsetof(MfsPrintQueryApi, origin), 16
    };
    if (!output || capacity < layout.size()) { return 1; }
    std::copy(layout.begin(), layout.end(), output);
    return 0;
}

namespace MfsPrintQueryReference { using MAA_CTRL_UNIT_NS::ScreencapBase; }
#undef MAA_CTRL_UNIT_NS_BEGIN
#undef MAA_CTRL_UNIT_NS_END
#define MAA_CTRL_UNIT_NS_BEGIN namespace MfsPrintQueryReference {
#define MAA_CTRL_UNIT_NS_END }
#define GetAncestor query_root
#define GetLastActivePopup query_popup
#define IsWindow query_valid
#define IsWindowVisible query_visible
#define ClientToScreen query_origin
#include MFS_UPSTREAM_PRINT_CPP

int32_t mfs_print_query_reference(void* main, uint32_t scenario,
    MfsParityImage* image, MfsPrintQueryReport* report)
{
    if (!image || !report) { return 1; }
    *image = {};
    *report = {};
    if (mfs_print_query_begin(main, scenario) != 0) { return 1; }
    int32_t status = 1;
    try {
        MfsPrintQueryReference::PrintWindowScreencap capture(static_cast<HWND>(main));
        auto pixels = capture.screencap();
        status = pixels ? mfs_parity::transfer_image(std::move(*pixels), image) : 1;
    }
    catch (const std::bad_alloc&) { status = 3; }
    catch (...) { status = 2; }
    if (mfs_print_query_end(report) != 0) { return 2; }
    return status;
}
