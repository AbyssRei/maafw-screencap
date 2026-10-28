// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// All SDK headers precede interposition; source-defined classes/helpers use
// their own namespace and cannot collide with the separate pristine reference.
#include "print-fault-probe.h"
#include "gdi-probe-internal.hpp"
#include "Base/UnitBase.h"
#include "Common/Conf.h"
#include "MaaUtils/Logger.h"
#include <new>
namespace MfsPrintFaultReference { using MAA_CTRL_UNIT_NS::ScreencapBase; }
namespace cv {
static void mfs_print_fault_cvtColor(const Mat& input, Mat& output, int code)
{
    mfs_parity::probe_conversion();
    cv::cvtColor(input, output, code);
}
}
#undef MAA_CTRL_UNIT_NS_BEGIN
#undef MAA_CTRL_UNIT_NS_END
#define MAA_CTRL_UNIT_NS_BEGIN namespace MfsPrintFaultReference {
#define MAA_CTRL_UNIT_NS_END }
#define GetDC mfs_probe_GetDC
#define CreateCompatibleDC mfs_probe_CreateCompatibleDC
#define GetClientRect mfs_probe_GetClientRect
#define CreateCompatibleBitmap mfs_probe_CreateCompatibleBitmap
#define SelectObject mfs_probe_SelectObject
#define PrintWindow mfs_probe_PrintWindow
#define GetDIBits mfs_probe_GetDIBits
#define DeleteObject mfs_probe_DeleteObject
#define DeleteDC mfs_probe_DeleteDC
#define ReleaseDC mfs_probe_ReleaseDC
#define cvtColor mfs_print_fault_cvtColor
#include MFS_UPSTREAM_PRINT_CPP
#undef cvtColor

int32_t mfs_print_fault_reference(void* hwnd, uint32_t failures, MfsParityImage* image, MfsGdiReport* report)
{
    if (!image || !report) { return 1; }
    *image = {};
    *report = {};
    if (mfs_gdi_probe_begin(failures) != 0) { return 1; }
    int32_t status = 1;
    try {
        MfsPrintFaultReference::PrintWindowScreencap capture(static_cast<HWND>(hwnd));
        auto pixels = capture.screencap();
        status = pixels ? mfs_parity::transfer_image(std::move(*pixels), image) : 1;
    }
    catch (const std::bad_alloc&) { status = 3; }
    catch (...) { status = 2; }
    if (mfs_gdi_probe_end(report) != 0) { return 2; }
    return status;
}
