// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "MaaUtils/SafeWindows.hpp"
#include <opencv2/core.hpp>
#include <opencv2/imgproc.hpp>
#include "gdi-probe.h"

namespace mfs_parity {
cv::Mat& image_pixels(void* owner);
int32_t transfer_image(cv::Mat pixels, MfsParityImage* output);
void probe_conversion();
}
HDC mfs_probe_GetDC(HWND hwnd);
HDC mfs_probe_CreateCompatibleDC(HDC dc);
BOOL mfs_probe_GetClientRect(HWND hwnd, RECT* rectangle);
HBITMAP mfs_probe_CreateCompatibleBitmap(HDC dc, int width, int height);
HGDIOBJ mfs_probe_SelectObject(HDC dc, HGDIOBJ object);
BOOL mfs_probe_BitBlt(HDC destination, int x, int y, int width, int height, HDC source, int sx, int sy, DWORD operation);
LONG mfs_probe_GetBitmapBits(HBITMAP bitmap, LONG length, void* output);
BOOL mfs_probe_DeleteObject(HGDIOBJ bitmap);
BOOL mfs_probe_DeleteDC(HDC dc);
int mfs_probe_ReleaseDC(HWND hwnd, HDC dc);
#if defined(MFS_UPSTREAM_PRINT_CPP)
BOOL mfs_probe_PrintWindow(HWND hwnd, HDC dc, UINT flags);
int mfs_probe_GetDIBits(HDC dc, HBITMAP bitmap, UINT start, UINT lines, void* output,
    BITMAPINFO* info, UINT colors);
#endif
