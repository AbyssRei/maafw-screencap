// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "gdi-parity.h"
#include <algorithm>
#include <array>
#include <limits>
#include <memory>
#include <new>
#include <opencv2/core.hpp>
#include <opencv2/imgproc.hpp>
#include "Screencap/GdiScreencap.h"

namespace {
struct Image { cv::Mat pixels; };

bool checked_span(size_t row, size_t stride, int32_t height, size_t& span)
{
    if (!row || height <= 0 || stride < row) { return false; }
    const size_t preceding_rows = static_cast<size_t>(height - 1);
    if (preceding_rows > (std::numeric_limits<size_t>::max() - row) / stride) { return false; }
    span = preceding_rows * stride + row;
    return true;
}

int32_t transfer(cv::Mat pixels, MfsParityImage* output)
{
    if (pixels.empty() || pixels.type() != CV_8UC3 || pixels.cols <= 0 || pixels.rows <= 0) { return 1; }
    const size_t width = static_cast<size_t>(pixels.cols);
    if (width > std::numeric_limits<size_t>::max() / 3) { return 1; }
    size_t span = 0;
    if (!checked_span(width * 3, pixels.step[0], pixels.rows, span)) { return 1; }
    auto owner = std::unique_ptr<Image>(new (std::nothrow) Image { std::move(pixels) });
    if (!owner) { return 3; }
    *output = { owner.get(), owner->pixels.data, span, owner->pixels.cols, owner->pixels.rows, owner->pixels.step[0] };
    owner.release();
    return 0;
}
}

int32_t mfs_gdi_parity_reference(void* hwnd, MfsParityImage* output)
{
    if (!output) { return 1; }
    *output = {};
    try {
        // Compile and invoke the exact original class, without PR or API edits.
        MAA_CTRL_UNIT_NS::GdiScreencap capture(static_cast<HWND>(hwnd));
        auto pixels = capture.screencap();
        return pixels ? transfer(std::move(*pixels), output) : 1;
    }
    catch (const std::bad_alloc&) { return 3; }
    catch (...) { return 2; }
}

int32_t mfs_gdi_parity_convert(const uint8_t* data, size_t length, int32_t width,
    int32_t height, size_t stride, uint32_t fault, MfsParityImage* output)
{
    if (!output) { return 1; }
    *output = {};
    if (!data || width <= 0 || fault > 1 || static_cast<size_t>(width) > std::numeric_limits<size_t>::max() / 4) { return 1; }
    size_t span = 0;
    if (!checked_span(static_cast<size_t>(width) * 4, stride, height, span) || length < span) { return 1; }
    try {
        if (fault == 1) { throw cv::Exception(cv::Error::StsBadArg, "injected parity conversion exception", "GDI parity", __FILE__, __LINE__); }
        // cv::Mat requires a non-const external pointer but cvtColor reads this
        // input only, never retaining it or aliasing it with the new BGR output.
        cv::Mat input(height, width, CV_8UC4, const_cast<uint8_t*>(data), stride);
        cv::Mat output_pixels;
        cv::cvtColor(input, output_pixels, cv::COLOR_BGRA2BGR);
        return transfer(std::move(output_pixels), output);
    }
    catch (const std::bad_alloc&) { return 3; }
    catch (...) { return 2; }
}

void mfs_gdi_parity_release(void* owner)
{
    delete static_cast<Image*>(owner);
}

int32_t mfs_gdi_parity_layout(uint64_t* output, size_t capacity)
{
    constexpr std::array<uint64_t, 13> layout {
        sizeof(MfsParityImage), alignof(MfsParityImage), offsetof(MfsParityImage, owner),
        offsetof(MfsParityImage, data), offsetof(MfsParityImage, length), offsetof(MfsParityImage, width),
        offsetof(MfsParityImage, height), offsetof(MfsParityImage, stride),
        sizeof(MfsParityWindow), alignof(MfsParityWindow), offsetof(MfsParityWindow, hwnd),
        offsetof(MfsParityWindow, width), offsetof(MfsParityWindow, height)
    };
    if (!output || capacity < layout.size()) { return 1; }
    std::copy(layout.begin(), layout.end(), output);
    return 0;
}

int32_t mfs_gdi_parity_paint(void* raw_hwnd, uint32_t phase)
{
    HWND hwnd = static_cast<HWND>(raw_hwnd);
    RECT client {};
    if (!hwnd || phase > 1 || !GetClientRect(hwnd, &client)) { return 1; }
    HDC dc = GetDC(hwnd);
    if (!dc) { return 1; }
    const int32_t width = client.right - client.left;
    const int32_t height = client.bottom - client.top;
    const std::array<RECT, 3> regions { RECT { 0, 0, width / 3, height },
        RECT { width / 3, 0, width, height / 2 }, RECT { width / 3, height / 2, width, height } };
    const std::array<COLORREF, 3> colors = phase == 0
        ? std::array<COLORREF, 3> { 0x001f53a7, 0x00b94d27, 0x003be281 }
        : std::array<COLORREF, 3> { 0x00e34219, 0x00578bc1, 0x009d23f5 };
    int32_t result = 0;
    for (size_t index = 0; index < regions.size(); ++index) {
        if (regions[index].right <= regions[index].left || regions[index].bottom <= regions[index].top) { continue; }
        HBRUSH brush = CreateSolidBrush(colors[index]);
        if (!brush) { result = 1; break; }
        const int filled = FillRect(dc, &regions[index], brush);
        const BOOL deleted = DeleteObject(brush);
        if (!filled || !deleted) { result = 1; break; }
    }
    if (!ReleaseDC(hwnd, dc)) { result = 1; }
    return result;
}

int32_t mfs_gdi_parity_window(int32_t width, int32_t height, MfsParityWindow* output)
{
    if (!output) { return 1; }
    *output = {};
    if (width < 0 || height < 0) { return 1; }
    HWND hwnd = CreateWindowExW(0, L"STATIC", L"mfs upstream GDI parity", WS_POPUP | WS_VISIBLE,
        80, 80, width, height, nullptr, nullptr, nullptr, nullptr);
    if (!hwnd) { return 1; }
    RECT client {};
    if (!GetClientRect(hwnd, &client) || mfs_gdi_parity_paint(hwnd, 0) != 0) { DestroyWindow(hwnd); return 1; }
    *output = { hwnd, client.right - client.left, client.bottom - client.top };
    return 0;
}

int32_t mfs_gdi_parity_destroy(void* hwnd)
{
    return hwnd && DestroyWindow(static_cast<HWND>(hwnd)) ? 0 : 1;
}

uint32_t mfs_gdi_parity_objects(void)
{
    return GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
}
