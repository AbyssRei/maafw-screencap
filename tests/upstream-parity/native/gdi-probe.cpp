// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "gdi-probe-internal.hpp"
#include <algorithm>
#include <array>
#include <cstring>

namespace {
struct Probe {
    bool active = false;
    uint32_t failures = 0;
    uint32_t selections = 0;
    HWND hwnd = nullptr;
    HDC dc = nullptr;
    HDC memory_dc = nullptr;
    HBITMAP bitmap = nullptr;
    HGDIOBJ previous = nullptr;
    MfsGdiReport report {};
};
thread_local Probe probe;
bool fails(uint32_t operation)
{
    if (!probe.active || !(probe.failures & (1u << (operation - 1)))) { return false; }
    SetLastError(0xe0f00000u | operation);
    return true;
}
void event(uint32_t operation, int32_t result)
{
    if (!probe.active) { return; }
    if (probe.report.count == MFS_GDI_PROBE_EVENTS) { probe.report.overflow = 1; return; }
    probe.report.events[probe.report.count++] = { operation, result, GetLastError() };
}
}

HDC mfs_probe_GetDC(HWND hwnd)
{
    SetLastError(0);
    HDC result = fails(MFS_GDI_GET_DC) ? nullptr : GetDC(hwnd);
    if (result) { probe.hwnd = hwnd; probe.dc = result; }
    event(MFS_GDI_GET_DC, result ? 1 : 0);
    return result;
}
HDC mfs_probe_CreateCompatibleDC(HDC dc)
{
    SetLastError(0);
    HDC result = fails(MFS_GDI_CREATE_DC) ? nullptr : CreateCompatibleDC(dc);
    if (result) { probe.memory_dc = result; }
    event(MFS_GDI_CREATE_DC, result ? 1 : 0);
    return result;
}
BOOL mfs_probe_GetClientRect(HWND hwnd, RECT* rectangle)
{
    SetLastError(0);
    BOOL result = fails(MFS_GDI_CLIENT_RECT) ? FALSE : GetClientRect(hwnd, rectangle);
    event(MFS_GDI_CLIENT_RECT, result);
    return result;
}
HBITMAP mfs_probe_CreateCompatibleBitmap(HDC dc, int width, int height)
{
    SetLastError(0);
    HBITMAP result = nullptr;
    if (!fails(MFS_GDI_CREATE_BITMAP)) {
        result = (probe.failures & MFS_GDI_BITMAP_24) ? CreateBitmap(width, height, 1, 24, nullptr)
            : CreateCompatibleBitmap(dc, width, height);
    }
    if (result) {
        probe.bitmap = result;
        const DWORD saved_error = GetLastError();
        BITMAP description {};
        if (GetObjectW(result, sizeof(description), &description) == sizeof(description)) {
            probe.report.bitmap_depth = description.bmBitsPixel;
        }
        SetLastError(saved_error);
    }
    event(MFS_GDI_CREATE_BITMAP, result ? 1 : 0);
    return result;
}
HGDIOBJ mfs_probe_SelectObject(HDC dc, HGDIOBJ object)
{
    const uint32_t operation = probe.selections++ == 0 ? MFS_GDI_SELECT : MFS_GDI_RESTORE;
    SetLastError(0);
    HGDIOBJ result = fails(operation) ? nullptr : SelectObject(dc, object);
    if (result && result != HGDI_ERROR) {
        if (operation == MFS_GDI_SELECT) { probe.previous = result; }
        else { probe.previous = nullptr; }
    }
    event(operation, result && result != HGDI_ERROR ? 1 : 0);
    return result;
}
BOOL mfs_probe_BitBlt(HDC destination, int x, int y, int width, int height, HDC source, int sx, int sy, DWORD operation)
{
    SetLastError(0);
    BOOL result = fails(MFS_GDI_COPY) ? FALSE : BitBlt(destination, x, y, width, height, source, sx, sy, operation);
    event(MFS_GDI_COPY, result);
    return result;
}
LONG mfs_probe_GetBitmapBits(HBITMAP bitmap, LONG length, void* output)
{
    SetLastError(0);
    LONG result = 0;
    if (!fails(MFS_GDI_READ)) {
        // The C++ source's nonzero-only check otherwise exposes an undefined
        // tail for short/24bpp data. Define it solely in this instrumented test.
        if (length > 0 && (probe.failures & (MFS_GDI_SHORT_READ | MFS_GDI_BITMAP_24))) {
            std::memset(output, 0xa5, static_cast<size_t>(length));
        }
        result = GetBitmapBits(bitmap, length, output);
        if (result > 1 && (probe.failures & MFS_GDI_SHORT_READ)) { result /= 2; }
    }
    event(MFS_GDI_READ, result);
    return result;
}
BOOL mfs_probe_DeleteObject(HGDIOBJ object)
{
    SetLastError(0);
    BOOL result = fails(MFS_GDI_DELETE_BITMAP) ? FALSE : DeleteObject(object);
    if (result && object == probe.bitmap) { probe.bitmap = nullptr; }
    event(MFS_GDI_DELETE_BITMAP, result);
    return result;
}
BOOL mfs_probe_DeleteDC(HDC dc)
{
    SetLastError(0);
    BOOL result = fails(MFS_GDI_DELETE_DC) ? FALSE : DeleteDC(dc);
    if (result && dc == probe.memory_dc) { probe.memory_dc = nullptr; probe.previous = nullptr; }
    event(MFS_GDI_DELETE_DC, result);
    return result;
}
int mfs_probe_ReleaseDC(HWND hwnd, HDC dc)
{
    SetLastError(0);
    int result = fails(MFS_GDI_RELEASE_DC) ? 0 : ReleaseDC(hwnd, dc);
    if (result && dc == probe.dc) { probe.dc = nullptr; }
    event(MFS_GDI_RELEASE_DC, result);
    return result;
}

namespace mfs_parity {
void probe_conversion()
{
    if (!probe.active) { return; }
    SetLastError(0);
    const bool failure = fails(MFS_GDI_CONVERT);
    event(MFS_GDI_CONVERT, failure ? 0 : 1);
    if (failure) { throw cv::Exception(cv::Error::StsBadArg, "injected GDI conversion", "GDI probe", __FILE__, __LINE__); }
}
}

int32_t mfs_gdi_probe_begin(uint32_t failures)
{
    constexpr uint32_t allowed = (1u << MFS_GDI_RELEASE_DC) - 1u | MFS_GDI_BITMAP_24 | MFS_GDI_SHORT_READ;
    if (probe.active || (failures & ~allowed)) { return 1; }
    probe = {};
    probe.active = true;
    probe.failures = failures;
    probe.report.before = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
    return 0;
}
int32_t mfs_gdi_probe_end(MfsGdiReport* output)
{
    if (!probe.active || !output) { return 1; }
    probe.report.after_capture = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
    // These are fixture recoveries after recording the implementation's own
    // attempts. They are not a hidden product retry or a successful close claim.
    if (probe.memory_dc && probe.previous && !SelectObject(probe.memory_dc, probe.previous)) { ++probe.report.fixture_errors; }
    if (probe.bitmap && !DeleteObject(probe.bitmap)) { ++probe.report.fixture_errors; }
    if (probe.memory_dc && !DeleteDC(probe.memory_dc)) { ++probe.report.fixture_errors; }
    if (probe.dc && !ReleaseDC(probe.hwnd, probe.dc)) { ++probe.report.fixture_errors; }
    probe.report.after_fixture = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
    *output = probe.report;
    probe = {};
    return 0;
}

extern "C" {
static void* api_get_dc(void* hwnd) { return mfs_probe_GetDC(static_cast<HWND>(hwnd)); }
static void* api_create_dc(void* dc) { return mfs_probe_CreateCompatibleDC(static_cast<HDC>(dc)); }
static int32_t api_client_rect(void* hwnd, void* rectangle) { return mfs_probe_GetClientRect(static_cast<HWND>(hwnd), static_cast<RECT*>(rectangle)); }
static void* api_create_bitmap(void* dc, int32_t width, int32_t height) { return mfs_probe_CreateCompatibleBitmap(static_cast<HDC>(dc), width, height); }
static void* api_select(void* dc, void* object) { return mfs_probe_SelectObject(static_cast<HDC>(dc), object); }
static int32_t api_copy(void* destination, int32_t width, int32_t height, void* source) { return mfs_probe_BitBlt(static_cast<HDC>(destination), 0, 0, width, height, static_cast<HDC>(source), 0, 0, SRCCOPY); }
static int32_t api_object(void* bitmap, int32_t length, void* output) {
    SetLastError(0);
    const int result = fails(MFS_GDI_OBJECT) ? 0 : GetObjectW(bitmap, length, output);
    event(MFS_GDI_OBJECT, result);
    return result;
}
static int32_t api_read(void* bitmap, int32_t length, void* output) { return mfs_probe_GetBitmapBits(static_cast<HBITMAP>(bitmap), length, output); }
static int32_t api_delete_bitmap(void* bitmap) { return mfs_probe_DeleteObject(bitmap); }
static int32_t api_delete_dc(void* dc) { return mfs_probe_DeleteDC(static_cast<HDC>(dc)); }
static int32_t api_release_dc(void* hwnd, void* dc) { return mfs_probe_ReleaseDC(static_cast<HWND>(hwnd), static_cast<HDC>(dc)); }
}

int32_t mfs_gdi_probe_api(MfsGdiApi* output)
{
    if (!output) { return 1; }
    *output = { api_get_dc, api_create_dc, api_client_rect, api_create_bitmap, api_select,
        api_copy, api_object, api_read, api_delete_bitmap, api_delete_dc, api_release_dc };
    return 0;
}
int32_t mfs_gdi_probe_layout(uint64_t* output, size_t capacity)
{
    constexpr std::array<uint64_t, 26> layout {
        sizeof(MfsGdiEvent), alignof(MfsGdiEvent), offsetof(MfsGdiEvent, result), offsetof(MfsGdiEvent, last_error),
        sizeof(MfsGdiReport), alignof(MfsGdiReport), offsetof(MfsGdiReport, events),
        sizeof(MfsGdiApi), alignof(MfsGdiApi), offsetof(MfsGdiApi, release_dc),
        MFS_GDI_GET_DC, MFS_GDI_CREATE_DC, MFS_GDI_CLIENT_RECT, MFS_GDI_CREATE_BITMAP, MFS_GDI_SELECT,
        MFS_GDI_COPY, MFS_GDI_OBJECT, MFS_GDI_READ, MFS_GDI_CONVERT, MFS_GDI_RESTORE,
        MFS_GDI_DELETE_BITMAP, MFS_GDI_DELETE_DC, MFS_GDI_RELEASE_DC, MFS_GDI_BITMAP_24,
        MFS_GDI_SHORT_READ, MFS_GDI_PROBE_EVENTS
    };
    if (!output || capacity < layout.size()) { return 1; }
    std::copy(layout.begin(), layout.end(), output);
    return 0;
}
