// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include <array>
#include <fstream>
#include <filesystem>
#include <iostream>
#include <stdexcept>
#include <opencv2/core.hpp>
#include "Screencap/GdiScreencap.h"

namespace {
struct Window {
    HWND handle = nullptr;
    ~Window() { if (handle) { DestroyWindow(handle); } }
};
}

int wmain(int argc, wchar_t** argv)
{
    try {
        if (argc != 2) {
            throw std::runtime_error("usage: mfs_reference_u <BGR output path>");
        }
        Window window { CreateWindowExW(0, L"STATIC", L"mfs unpatched reference", WS_POPUP | WS_VISIBLE,
            80, 80, 32, 24, nullptr, nullptr, nullptr, nullptr) };
        if (!window.handle) {
            throw std::runtime_error("CreateWindowExW failed");
        }
        RECT rectangle {};
        if (!GetClientRect(window.handle, &rectangle)) {
            throw std::runtime_error("GetClientRect failed");
        }
        HDC dc = GetDC(window.handle);
        if (!dc) {
            throw std::runtime_error("GetDC failed");
        }
        HBRUSH brush = CreateSolidBrush(RGB(0x11, 0x22, 0x33));
        int filled = brush ? FillRect(dc, &rectangle, brush) : 0;
        BOOL deleted = brush ? DeleteObject(brush) : TRUE;
        int released = ReleaseDC(window.handle, dc);
        if (!filled || !deleted || !released) {
            throw std::runtime_error("controlled window paint or GDI cleanup failed");
        }
        // Invoke the original fixed C++ class and its original OpenCV conversion.
        MAA_CTRL_UNIT_NS::GdiScreencap capture(window.handle);
        auto image = capture.screencap();
        if (!image || image->empty() || image->type() != CV_8UC3) {
            throw std::runtime_error("fixed upstream GDI capture failed");
        }
        if (!DestroyWindow(window.handle)) { throw std::runtime_error("controlled window destruction failed"); }
        window.handle = nullptr;
        const std::array<uint8_t, 3> expected { 0x33, 0x22, 0x11 };
        for (int y = 0; y < image->rows; ++y) {
            for (int x = 0; x < image->cols; ++x) {
                auto pixel = image->at<cv::Vec3b>(y, x);
                if (pixel[0] != expected[0] || pixel[1] != expected[1] || pixel[2] != expected[2]) {
                    throw std::runtime_error("fixed upstream pixels differ from controlled window");
                }
            }
        }
        std::ofstream output(std::filesystem::path(argv[1]), std::ios::binary | std::ios::trunc);
        if (!output) { throw std::runtime_error("cannot create BGR output"); }
        for (int y = 0; y < image->rows; ++y) {
            output.write(reinterpret_cast<const char*>(image->ptr(y)), static_cast<std::streamsize>(image->cols) * 3);
        }
        output.close();
        if (!output) { throw std::runtime_error("BGR output write failed"); }
        std::cout << "unpatched GDI: " << image->cols << "x" << image->rows << " BGR8, every target pixel verified\n";
        return 0;
    }
    catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
    catch (...) {
        std::cerr << "unknown native exception\n";
        return 1;
    }
}
