# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
cmake_minimum_required(VERSION 4.2)
project(png_codec_probe LANGUAGES C)
set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)
find_package(PNG REQUIRED)
add_executable(png_codec_probe main.c)
target_link_libraries(png_codec_probe PRIVATE PNG::PNG)
if(UNIX AND NOT APPLE)
    target_link_libraries(png_codec_probe PRIVATE m)
endif()
