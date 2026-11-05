// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Acceptance-only screen DC observations. No capture backend/SDK is added.
// The independent memory control uses GetPixel/COLORREF, including CLR_INVALID:
// https://learn.microsoft.com/windows/win32/api/wingdi/nf-wingdi-getpixel
// GetDC(nullptr)/ReleaseDC are paired on this same native caller thread:
// https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getdc
#pragma once
#include <windows.h>
#include <array>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <limits>
#include <stdexcept>
#include <string>
#include <vector>

namespace mfs_screen_witness {
constexpr int WIDTH = 96, HEIGHT = 64;
constexpr size_t PIXELS = WIDTH * HEIGHT, BYTES = PIXELS * 3;
struct Copy {
    int blit = 0, flush = 0, lines = 0, restored = 0, bitmap_deleted = 0, memory_deleted = 0;
    bool layout = false;
};
inline Copy copy(HDC device, HDC source, int x, int y, int width, int height, DWORD operation, std::vector<uint8_t>& packed)
{
    Copy result;
    if (width <= 0 || width > WIDTH || height <= 0 || height > HEIGHT
        || packed.size() != static_cast<size_t>(width) * static_cast<size_t>(height) * 4) { return result; }
    const auto memory = CreateCompatibleDC(device);
    // A DDB from the device DC, not the default monochrome memory bitmap.
    const auto bitmap = memory ? CreateCompatibleBitmap(device, width, height) : nullptr;
    const auto previous = bitmap ? SelectObject(memory, bitmap) : nullptr;
    const bool selected = previous && previous != HGDI_ERROR;
    if (selected) {
        result.blit = BitBlt(memory, 0, 0, width, height, source, x, y, operation) ? 1 : 0;
        result.flush = GdiFlush() ? 1 : 0;
        result.restored = SelectObject(memory, previous) == bitmap ? 1 : 0;
    }
    BITMAPINFO info {}; info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    info.bmiHeader.biWidth = width; info.bmiHeader.biHeight = -height; info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32; info.bmiHeader.biCompression = BI_RGB;
    // GetDIBits requires that the DDB no longer be selected in any DC.
    // https://learn.microsoft.com/windows/win32/api/wingdi/nf-wingdi-getdibits
    if (result.blit && result.flush && result.restored) {
        result.lines = GetDIBits(device, bitmap, 0, static_cast<UINT>(height), packed.data(), &info, DIB_RGB_COLORS);
        result.layout = info.bmiHeader.biWidth == width && info.bmiHeader.biHeight == -height
            && info.bmiHeader.biPlanes == 1 && info.bmiHeader.biBitCount == 32 && info.bmiHeader.biCompression == BI_RGB;
    }
    result.bitmap_deleted = bitmap && DeleteObject(bitmap) ? 1 : 0;
    result.memory_deleted = memory && DeleteDC(memory) ? 1 : 0;
    return result;
}
using Geometry = std::array<int64_t, 12>;
inline int64_t clock()
{
    LARGE_INTEGER value {};
    if (!QueryPerformanceCounter(&value)) { throw std::runtime_error("screen witness QPC failed"); }
    return value.QuadPart;
}
inline Geometry geometry(HWND hwnd)
{
    RECT outer {}, client {}; POINT origin {};
    if (!GetWindowRect(hwnd, &outer) || !GetClientRect(hwnd, &client) || !ClientToScreen(hwnd, &origin)) {
        throw std::runtime_error("screen witness geometry query failed");
    }
    SetLastError(0); const auto style = GetWindowLongPtrW(hwnd, GWL_STYLE);
    if (!style && GetLastError()) { throw std::runtime_error("screen witness style query failed"); }
    SetLastError(0); const auto extended = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if (!extended && GetLastError()) { throw std::runtime_error("screen witness extended style query failed"); }
    if (client.left != 0 || client.top != 0 || client.right != WIDTH || client.bottom != HEIGHT
        || static_cast<int64_t>(origin.x) + WIDTH - 1 > (std::numeric_limits<int>::max)()
        || static_cast<int64_t>(origin.y) + HEIGHT - 1 > (std::numeric_limits<int>::max)()) {
        throw std::runtime_error("screen witness complete target coordinates differ");
    }
    return { static_cast<uint32_t>(style), outer.left, outer.top, outer.right, outer.bottom,
        client.left, client.top, client.right, client.bottom, origin.x, origin.y, static_cast<uint32_t>(extended) };
}
struct Read {
    uint64_t slot = 0, side = 0, thread = 0, release_thread = 0;
    int64_t begin = 0, end = 0;
    int map_mode = 0, released = 0;
    Copy copied;
    Geometry before {}, after {};
    std::vector<uint8_t> pixels;
};
inline Read read(HWND hwnd, uint64_t slot, uint64_t side)
{
    Read result; result.slot = slot; result.side = side; result.thread = GetCurrentThreadId();
    result.before = geometry(hwnd);
    // Allocate before acquiring the common DC; the open-DC region performs no
    // allocation, throwing C++ operation, source callback, repaint or message pump.
    result.pixels.resize(BYTES);
    std::vector<uint8_t> packed(PIXELS * 4);
    result.begin = clock();
    const auto dc = GetDC(nullptr);
    if (!dc) { throw std::runtime_error("screen witness GetDC failed"); }
    result.map_mode = GetMapMode(dc);
    POINT viewport {}, window {};
    const bool coordinates = result.map_mode == MM_TEXT && GetViewportOrgEx(dc, &viewport)
        && GetWindowOrgEx(dc, &window) && viewport.x == 0 && viewport.y == 0 && window.x == 0 && window.y == 0;
    if (coordinates) {
        result.copied = copy(dc, dc, static_cast<int>(result.before[9]), static_cast<int>(result.before[10]),
            WIDTH, HEIGHT, SRCCOPY | CAPTUREBLT, packed);
    }
    result.release_thread = GetCurrentThreadId();
    result.released = ReleaseDC(nullptr, dc);
    result.end = clock(); result.after = geometry(hwnd);
    const auto& c = result.copied;
    if (!coordinates || !c.blit || !c.flush || c.lines != HEIGHT || !c.layout || !c.restored
        || !c.bitmap_deleted || !c.memory_deleted || result.released != 1 || result.thread != result.release_thread) {
        throw std::runtime_error("screen witness failed coordinates=" + std::to_string(coordinates)
            + " blit=" + std::to_string(c.blit) + " flush=" + std::to_string(c.flush) + " lines=" + std::to_string(c.lines)
            + " layout=" + std::to_string(c.layout) + " restored=" + std::to_string(c.restored)
            + " bitmap_deleted=" + std::to_string(c.bitmap_deleted) + " memory_deleted=" + std::to_string(c.memory_deleted)
            + " ReleaseDC=" + std::to_string(result.released));
    }
    for (size_t i = 0; i < PIXELS; ++i) {
        for (size_t channel = 0; channel < 3; ++channel) { result.pixels[i * 3 + channel] = packed[i * 4 + channel]; }
    }
    return result;
}
inline void control(const std::filesystem::path& root)
{
    constexpr std::array<std::array<uint8_t, 3>, 6> colors {{
        {31,83,167}, {185,77,39}, {59,226,129}, {0,0,0}, {255,255,255}, {227,66,25}
    }};
    std::array<COLORREF, 6> actual {};
    std::vector<uint8_t> packed(24);
    const auto device = GetDC(nullptr);
    const auto dc = CreateCompatibleDC(nullptr);
    BITMAPINFO info {}; info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    info.bmiHeader.biWidth = 3; info.bmiHeader.biHeight = -2; info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32; info.bmiHeader.biCompression = BI_RGB;
    void* bits = nullptr;
    const auto bitmap = dc ? CreateDIBSection(dc, &info, DIB_RGB_COLORS, &bits, nullptr, 0) : nullptr;
    const auto previous = bitmap ? SelectObject(dc, bitmap) : nullptr;
    const bool selected = previous && previous != HGDI_ERROR;
    bool pixels = selected && bits, clipped = false;
    Copy copied;
    if (pixels) {
        auto* bytes = static_cast<uint8_t*>(bits);
        for (size_t i = 0; i < colors.size(); ++i) {
            for (size_t c = 0; c < 3; ++c) { bytes[i * 4 + c] = colors[i][c]; }
            bytes[i * 4 + 3] = static_cast<uint8_t>(17 + i);
        }
        for (size_t i = 0; i < colors.size(); ++i) {
            actual[i] = GetPixel(dc, static_cast<int>(i % 3), static_cast<int>(i / 3));
            pixels &= actual[i] != CLR_INVALID && GetBValue(actual[i]) == colors[i][0]
                && GetGValue(actual[i]) == colors[i][1] && GetRValue(actual[i]) == colors[i][2];
        }
        clipped = GetPixel(dc, -1, 0) == CLR_INVALID;
        if (device) { copied = copy(device, dc, 0, 0, 3, 2, SRCCOPY, packed); }
        for (size_t i = 0; i < colors.size(); ++i) {
            for (size_t channel = 0; channel < 3; ++channel) { pixels &= packed[i * 4 + channel] == colors[i][channel]; }
        }
    }
    const auto restored_object = selected ? SelectObject(dc, previous) : nullptr;
    const bool restored = selected && restored_object == bitmap;
    const bool bitmap_deleted = bitmap && DeleteObject(bitmap);
    const bool dc_deleted = dc && DeleteDC(dc);
    const int screen_released = device ? ReleaseDC(nullptr, device) : 0;
    // No native resource remains open when a stream or exception can allocate.
    if (!pixels || !clipped || !restored || !bitmap_deleted || !dc_deleted || screen_released != 1
        || !copied.blit || !copied.flush || copied.lines != 2 || !copied.layout || !copied.restored
        || !copied.bitmap_deleted || !copied.memory_deleted) {
        throw std::runtime_error("screen witness memory control failed pixels=" + std::to_string(pixels)
            + " clipped=" + std::to_string(clipped) + " restored=" + std::to_string(restored)
            + " bitmap_deleted=" + std::to_string(bitmap_deleted) + " dc_deleted=" + std::to_string(dc_deleted)
            + " blit=" + std::to_string(copied.blit) + " lines=" + std::to_string(copied.lines)
            + " screen_released=" + std::to_string(screen_released));
    }
    std::ofstream out(root / "pixel-control.tsv"); out.exceptions(std::ios::badbit | std::ios::failbit);
    for (size_t i = 0; i < colors.size(); ++i) {
        out << i << '\t' << static_cast<unsigned>(colors[i][0]) << '\t' << static_cast<unsigned>(colors[i][1])
            << '\t' << static_cast<unsigned>(colors[i][2]) << '\t' << actual[i];
        for (size_t channel = 0; channel < 3; ++channel) { out << '\t' << static_cast<unsigned>(packed[i * 4 + channel]); }
        out << '\n';
    }
    out.close();
    std::ofstream summary(root / "control-summary.txt"); summary.exceptions(std::ios::badbit | std::ios::failbit);
    summary << "schema=2\nwidth=3\nheight=2\ngetpixel_calls=7\nvalid_pixels=6\nclipped_rejected=1\nbitblt_calls=1\ngetdibits_rows=2\nbitmap_restored=2\nbitmap_deleted=2\nmemory_dc_deleted=2\nscreen_dc_released=1\n";
    summary.close();
}
inline void save(const std::vector<Read>& reads, const std::filesystem::path& root)
{
    if (reads.size() != 54) { throw std::runtime_error("screen witness read inventory differs"); }
    std::ofstream rows(root / "observations.tsv"); rows.exceptions(std::ios::badbit | std::ios::failbit);
    for (const auto& item : reads) {
        rows << item.slot << '\t' << item.side << '\t' << item.thread << '\t' << item.release_thread
            << '\t' << item.begin << '\t' << item.end << '\t' << item.map_mode << '\t' << item.copied.blit
            << '\t' << item.copied.flush << '\t' << item.copied.lines << '\t' << item.copied.restored
            << '\t' << item.copied.bitmap_deleted << '\t' << item.copied.memory_deleted << '\t' << item.released;
        for (const auto v : item.before) { rows << '\t' << v; }
        for (const auto v : item.after) { rows << '\t' << v; }
        rows << '\n';
        std::ofstream bytes(root / ("screen-" + std::to_string(item.slot) + "-" + std::to_string(item.side) + ".bgr"), std::ios::binary);
        bytes.exceptions(std::ios::badbit | std::ios::failbit);
        if (item.pixels.size() != BYTES) { throw std::runtime_error("screen witness payload size differs"); }
        bytes.write(reinterpret_cast<const char*>(item.pixels.data()), BYTES); bytes.close();
    }
    rows.close();
    std::ofstream summary(root / "summary.txt"); summary.exceptions(std::ios::badbit | std::ios::failbit);
    summary << "schema=2\nreads=54\nwidth=96\nheight=64\nrow_bytes=288\npixels_per_read=6144\nbitblt_calls=54\ngetdibits_calls=54\ngetdibits_rows=3456\nbitmap_restored=54\nbitmap_deleted=54\nmemory_dc_deleted=54\ndc_acquired=54\ndc_released=54\nsource_calls=27\nsource_bodies_unchanged=1\nadded_false_setters=0\ntarget_inset=0\natomic_screen_snapshot=0\nproducer_proved=0\noriginal_failure_reclassified=0\nfull_P1_acceptance=0\n";
    summary.close();
}
} // namespace mfs_screen_witness
