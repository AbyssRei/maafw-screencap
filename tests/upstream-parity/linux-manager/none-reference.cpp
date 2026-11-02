// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// The unedited F None class and exact extracted screenshot base, consumed
// through real C++ virtual calls. No scripted capture, daemon or display.
#include "Screencap/NoneScreencap.h"
#include <array>
#include <cstdint>
#include <iostream>
#include <limits>
#include <memory>
#include <stdexcept>
#include <type_traits>
#include <vector>

namespace {
using Base = MAA_CTRL_UNIT_NS::ScreencapBase;
using None = MAA_CTRL_UNIT_NS::NoneScreencap;
static_assert(std::has_virtual_destructor_v<Base> && std::is_base_of_v<Base, None>);

void require(bool value, const char* message)
{
    if (!value) { throw std::runtime_error(message); }
}

std::vector<uint8_t> bytes(const cv::Mat& image)
{
    if (image.empty()) { return {}; }
    require(image.dims == 2 && image.rows > 0 && image.cols > 0 && image.type() == CV_8UC3,
        "unexpected owned None caller layout");
    const auto width = static_cast<size_t>(image.cols), height = static_cast<size_t>(image.rows);
    require(width <= (std::numeric_limits<size_t>::max)() / 3, "None caller row overflow");
    const auto row = width * 3;
    require(height <= (std::numeric_limits<size_t>::max)() / row && image.step[0] >= row,
        "None caller span overflow");
    std::vector<uint8_t> result;
    result.reserve(row * height);
    for (int y = 0; y < image.rows; ++y) {
        // Owned, initialized native Mat rows. Do not read ROI-external padding.
        const auto* begin = image.ptr<uint8_t>(y);
        result.insert(result.end(), begin, begin + row);
    }
    return result;
}

struct State {
    std::array<uintptr_t, 14> header;
    std::vector<uint8_t> pixels;
    std::vector<uint8_t> backing;
    bool operator==(const State&) const = default;
};

State state(const cv::Mat& image, const cv::Mat& backing)
{
    require(image.dims >= 0 && image.rows >= 0 && image.cols >= 0,
        "negative None caller header");
    require(!image.u || image.u->refcount > 0, "invalid None caller owner count");
    return { {
        reinterpret_cast<uintptr_t>(image.data), reinterpret_cast<uintptr_t>(image.datastart),
        reinterpret_cast<uintptr_t>(image.dataend), reinterpret_cast<uintptr_t>(image.datalimit),
        reinterpret_cast<uintptr_t>(image.u), static_cast<uintptr_t>(image.flags),
        static_cast<uintptr_t>(image.dims), static_cast<uintptr_t>(image.rows),
        static_cast<uintptr_t>(image.cols), image.step[0], image.step[1],
        static_cast<uintptr_t>(image.type()), static_cast<uintptr_t>(image.u ? image.u->refcount : 0),
        static_cast<uintptr_t>(image.isSubmatrix()) }, bytes(image), bytes(backing) };
}

void fill(cv::Mat& image)
{
    for (int y = 0; y < image.rows; ++y) {
        for (int x = 0; x < image.cols; ++x) {
            image.at<cv::Vec3b>(y, x) = cv::Vec3b(
                static_cast<uint8_t>(13 + x * 7), static_cast<uint8_t>(29 + y * 11),
                static_cast<uint8_t>(47 + x * 3 + y * 5));
        }
    }
}

struct Delete {
    unsigned* completed;
    void operator()(Base* source) const
    {
        delete source; // The actual original virtual destructor chain.
        ++*completed;
    }
};
}

int main(int argc, char**)
{
    if (argc != 1) { return 64; }
    try {
        unsigned initialized = 0, rejected = 0, inactive = 0, deleted = 0, unchanged = 0;
        for (unsigned shape = 0; shape < 4; ++shape) {
            cv::Mat image, backing;
            if (shape == 1) {
                image = cv::Mat(2, 3, CV_8UC3); fill(image); image.release();
                require(image.empty() && image.type() == CV_8UC3, "released header premise differs");
            } else if (shape == 2) {
                image = cv::Mat(2, 3, CV_8UC3); fill(image);
                require(image.isContinuous() && image.u->refcount == 1, "packed caller premise differs");
            } else if (shape == 3) {
                backing = cv::Mat(5, 7, CV_8UC3); fill(backing);
                image = backing(cv::Rect(2, 1, 3, 2));
                require(!image.isContinuous() && image.isSubmatrix() && image.u == backing.u
                    && image.data != backing.data && image.u->refcount == 2,
                    "noncontiguous ROI caller premise differs");
            }
            const auto before = state(image, backing);
            std::unique_ptr<Base, Delete> source(new None, Delete { &deleted });
            require(source->init(), "original inherited None init failed"); ++initialized;
            require(!source->screencap(image), "original None unexpectedly captured"); ++rejected;
            require(state(image, backing) == before, "original None changed first caller"); ++unchanged;
            source->inactive(); ++inactive;
            require(!source->screencap(image), "original inactive None unexpectedly captured"); ++rejected;
            require(state(image, backing) == before, "original None changed inactive caller"); ++unchanged;
            source.reset();
            require(state(image, backing) == before, "original None destruction changed held caller"); ++unchanged;
            std::cout << "original None caller shape=" << shape << " false_calls=2 header_and_full_pixels=unchanged held_after_virtual_delete=1\n";
        }
        require(initialized == 4 && rejected == 8 && inactive == 4 && deleted == 4 && unchanged == 12,
            "incomplete original None caller matrix");
        std::cout << "original NoneScreencap cases=4 virtual_init=4 false_calls=8 inactive=4 virtual_deletes=4 caller_checks=12 held_callers=4 source_bodies_unchanged=1 native_captures=0 full_P1_acceptance=0 passed\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "original None caller failure=" << error.what() << '\n'; return 3;
    }
}
