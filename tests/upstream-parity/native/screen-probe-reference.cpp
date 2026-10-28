// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Exact original body with only the resource/read/conversion API observation
// boundary changed. Window queries/helpers remain native and uninstrumented.
#include "screen-parity.h"
#include "gdi-probe-internal.hpp"
#include "Base/UnitBase.h"
#include "Common/Conf.h"
#include "MaaUtils/Logger.h"
#include <chrono>
#include <thread>
#include <new>
namespace MfsScreenProbeReference { using MAA_CTRL_UNIT_NS::ScreencapBase; }
namespace cv {
static void mfs_screen_probe_cvtColor(const Mat& input, Mat& output, int code)
{
    mfs_parity::probe_conversion();
    cv::cvtColor(input, output, code);
}
}
#undef MAA_CTRL_UNIT_NS_BEGIN
#undef MAA_CTRL_UNIT_NS_END
#define MAA_CTRL_UNIT_NS_BEGIN namespace MfsScreenProbeReference {
#define MAA_CTRL_UNIT_NS_END }
#define GetDC mfs_probe_GetDC
#define CreateCompatibleDC mfs_probe_CreateCompatibleDC
#define CreateCompatibleBitmap mfs_probe_CreateCompatibleBitmap
#define SelectObject mfs_probe_SelectObject
#define BitBlt mfs_probe_BitBlt
#define GetBitmapBits mfs_probe_GetBitmapBits
#define DeleteObject mfs_probe_DeleteObject
#define DeleteDC mfs_probe_DeleteDC
#define ReleaseDC mfs_probe_ReleaseDC
#define cvtColor mfs_screen_probe_cvtColor
#include MFS_UPSTREAM_SCREEN_CPP
#undef cvtColor

int32_t mfs_screen_probe_reference(void* window, uint32_t failures, MfsParityImage* image, MfsGdiReport* report)
{
    if (!image || !report) { return 1; }
    *image = {}; *report = {};
    if (mfs_gdi_probe_begin(failures) != 0) { return 1; }
    int32_t status = 1;
    try {
        MfsScreenProbeReference::ScreenDCScreencap capture(static_cast<HWND>(window));
        auto pixels = capture.screencap();
        status = pixels ? mfs_parity::transfer_image(std::move(*pixels), image) : 1;
    }
    catch (const std::bad_alloc&) { status = 3; }
    catch (...) { status = 2; }
    if (mfs_gdi_probe_end(report) != 0) { return 2; }
    return status;
}
int32_t mfs_screen_probe_copy(void* destination, int32_t width, int32_t height, void* source, int32_t x, int32_t y)
{
    return mfs_probe_BitBlt(static_cast<HDC>(destination), 0, 0, width, height, static_cast<HDC>(source), x, y, SRCCOPY);
}
