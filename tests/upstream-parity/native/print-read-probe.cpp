// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Diagnostic TU only: preserve the original function body and isolate all its
// source-defined helpers/classes. The pristine reference is compiled separately.
#include "print-parity.h"
#include "gdi-probe-internal.hpp"
#include "Base/UnitBase.h"
#include "Common/Conf.h"
#include "MaaUtils/Logger.h"
#include <array>
#include <algorithm>
#include <cstring>
#include <new>
#include <climits>

namespace {
struct Context { HDC dc = nullptr; HGDIOBJ previous = nullptr; HBITMAP bitmap = nullptr; };
struct Probe {
    bool active = false;
    uint32_t mode = 0;
    size_t current = 0;
    std::array<Context, 2> contexts {};
    std::array<uint64_t, 36> report {};
};
thread_local Probe probe;
uint64_t colored(const void* data, size_t length)
{
    uint64_t count = 0;
    const auto* bytes = static_cast<const uint8_t*>(data);
    for (size_t i = 0; i < length; ++i) { if (i % 4 != 3 && bytes[i]) { ++count; } }
    return count;
}
HDC observed_create_dc(HDC source)
{
    HDC result = CreateCompatibleDC(source);
    if (probe.active) {
        const size_t slot = static_cast<size_t>(probe.report[3]++);
        if (slot < probe.contexts.size()) { probe.current = slot; probe.contexts[slot].dc = result; }
        else { probe.report[1] = 1; }
    }
    return result;
}
HGDIOBJ observed_select(HDC dc, HGDIOBJ object)
{
    HGDIOBJ previous = SelectObject(dc, object);
    auto& context = probe.contexts[probe.current];
    if (probe.active && context.dc == dc && !context.previous && previous && previous != HGDI_ERROR) {
        context.previous = previous;
        context.bitmap = static_cast<HBITMAP>(object);
    }
    return previous;
}
int observed_dibits(HDC dc, HBITMAP bitmap, UINT start, UINT lines, void* output,
    BITMAPINFO* info, UINT colors)
{
    const size_t slot = static_cast<size_t>(probe.report[0]++);
    if (!probe.active || slot >= 2 || info->bmiHeader.biWidth <= 0
        || info->bmiHeader.biHeight >= 0 || info->bmiHeader.biBitCount != 32
        || info->bmiHeader.biHeight == INT_MIN) {
        probe.report[1] = 1;
        return GetDIBits(dc, bitmap, start, lines, output, info, colors);
    }
    const uint64_t width = static_cast<uint64_t>(info->bmiHeader.biWidth);
    const uint64_t height = static_cast<uint64_t>(-info->bmiHeader.biHeight);
    const uint64_t size = width * height * 4;
    if (!output || size > 4096) { probe.report[1] = 1; return 0; }
    auto* record = probe.report.data() + 4 + slot * 16;
    record[0] = width; record[1] = height;
    auto& context = probe.contexts[probe.current];
    // Define the entire diagnostic source input even if native reads are short.
    // This does not change pixels written by a complete successful read.
    std::memset(output, 0, static_cast<size_t>(size));
    if (probe.mode == 1) { SelectObject(dc, context.previous); }
    record[2] = GetCurrentObject(dc, OBJ_BITMAP) == bitmap;
    const int first = GetDIBits(dc, bitmap, start, lines, output, info, colors);
    const DWORD first_error = GetLastError();
    record[3] = static_cast<uint64_t>(static_cast<int64_t>(first));
    record[4] = colored(output, static_cast<size_t>(size));
    if (probe.mode == 1) { SelectObject(dc, bitmap); }
    record[13] = first_error;
    record[14] = reinterpret_cast<uintptr_t>(dc);
    record[15] = reinterpret_cast<uintptr_t>(bitmap);
    // Do not add further native reads/delays to a normally rendered capture.
    // These fixture palettes have no zero RGB components. Only an anomalous
    // first result triggers the additional same-bitmap diagnostic observations.
    if (record[4] == width * height * 3) { SetLastError(first_error); return first; }
    std::array<uint32_t, 1024> extra {};
    const int repeated = GetDIBits(dc, bitmap, start, lines, extra.data(), info, colors);
    record[5] = static_cast<uint64_t>(static_cast<int64_t>(repeated));
    record[6] = colored(extra.data(), static_cast<size_t>(size));
    extra.fill(0);
    BITMAP description {};
    if (GetObjectW(bitmap, sizeof(description), &description) == sizeof(description)
        && description.bmPlanes == 1 && description.bmBitsPixel == 32
        && description.bmWidthBytes == static_cast<LONG>(width * 4)) {
        record[7] = static_cast<uint64_t>(static_cast<int64_t>(GetBitmapBits(bitmap, static_cast<LONG>(size), extra.data())));
        record[8] = colored(extra.data(), static_cast<size_t>(size));
    }
    const HGDIOBJ detached = SelectObject(dc, context.previous);
    record[9] = detached && detached != HGDI_ERROR;
    extra.fill(0);
    record[10] = static_cast<uint64_t>(static_cast<int64_t>(GetDIBits(dc, bitmap, start, lines, extra.data(), info, colors)));
    record[11] = colored(extra.data(), static_cast<size_t>(size));
    const HGDIOBJ restored = SelectObject(dc, bitmap);
    record[12] = restored && restored != HGDI_ERROR;
    SetLastError(first_error);
    return first;
}
}

namespace MfsPrintReadProbe { using MAA_CTRL_UNIT_NS::ScreencapBase; }
#undef MAA_CTRL_UNIT_NS_BEGIN
#undef MAA_CTRL_UNIT_NS_END
#define MAA_CTRL_UNIT_NS_BEGIN namespace MfsPrintReadProbe {
#define MAA_CTRL_UNIT_NS_END }
#define CreateCompatibleDC observed_create_dc
#define SelectObject observed_select
#define GetDIBits observed_dibits
#include MFS_UPSTREAM_PRINT_CPP

int32_t mfs_print_probe_reference(void* hwnd, uint32_t mode, MfsParityImage* image,
    uint64_t* observations, size_t capacity)
{
    if (!image || !observations || capacity < probe.report.size() || probe.active || mode > 1) { return 1; }
    *image = {};
    std::fill_n(observations, 36, 0);
    probe = {};
    probe.active = true;
    probe.mode = mode;
    probe.report[2] = mode;
    int32_t status = 1;
    try {
        MfsPrintReadProbe::PrintWindowScreencap capture(static_cast<HWND>(hwnd));
        auto pixels = capture.screencap();
        status = pixels ? mfs_parity::transfer_image(std::move(*pixels), image) : 1;
    }
    catch (const std::bad_alloc&) { status = 3; }
    catch (...) { status = 2; }
    std::copy(probe.report.begin(), probe.report.end(), observations);
    probe = {};
    return status;
}
