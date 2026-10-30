cmake_minimum_required(VERSION 4.2)
project(mfs_macos_source_context LANGUAGES CXX OBJCXX)
if(NOT APPLE OR CMAKE_CROSSCOMPILING OR NOT CMAKE_SYSTEM_PROCESSOR MATCHES "^(arm64|aarch64)$" OR
   NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
    message(FATAL_ERROR "This source context requires native macOS arm64")
endif()
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_OBJCXX_STANDARD 23)
set(CMAKE_OBJCXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_SCAN_FOR_MODULES OFF)
set(OpenCV_STATIC ON)
find_package(OpenCV REQUIRED COMPONENTS core imgproc)
find_package(Threads REQUIRED)
include("${CMAKE_CURRENT_SOURCE_DIR}/../../../build-support/macos-base-source.cmake")
include("${CMAKE_CURRENT_SOURCE_DIR}/../../../build-support/macos-manager-source.cmake")
add_subdirectory("${CMAKE_CURRENT_SOURCE_DIR}/../manager-metadata" metadata)
set(_mfs_root "${MFS_UPSTREAM_ROOT}/source/MaaMacOSControlUnit")
add_executable(mfs_macos_source_context source-context.mm
    "${_mfs_root}/Screencap/ScreenCaptureKitScreencap.mm")
target_compile_options(mfs_macos_source_context PRIVATE -fno-objc-arc -fblocks -Wall -Wextra)
target_include_directories(mfs_macos_source_context PRIVATE
    "${CMAKE_CURRENT_BINARY_DIR}" "${CMAKE_CURRENT_BINARY_DIR}/overlay" "${CMAKE_CURRENT_SOURCE_DIR}/overlay"
    "${MFS_UPSTREAM_ROOT}/include"
    "${_mfs_root}" ${OpenCV_INCLUDE_DIRS})
target_link_libraries(mfs_macos_source_context PRIVATE Threads::Threads ${OpenCV_LIBS})
find_library(_mfs_proc NAMES proc REQUIRED)
target_link_libraries(mfs_macos_source_context PRIVATE "${_mfs_proc}")
add_library(mfs_macos_opencv SHARED ../linux-pipewire/conversion-adapter.cpp acceptance-evidence.cpp manager-script.cpp)
target_include_directories(mfs_macos_opencv PRIVATE ${OpenCV_INCLUDE_DIRS}
    "${CMAKE_CURRENT_BINARY_DIR}" "${CMAKE_CURRENT_BINARY_DIR}/overlay"
    "${CMAKE_CURRENT_SOURCE_DIR}/overlay" "${MFS_UPSTREAM_ROOT}/include")
target_link_libraries(mfs_macos_opencv PRIVATE Threads::Threads ${OpenCV_LIBS} "${_mfs_proc}")
set_target_properties(mfs_macos_opencv PROPERTIES CXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN YES)
foreach(_framework AppKit ApplicationServices CoreFoundation CoreMedia CoreVideo ScreenCaptureKit)
    find_library(_mfs_${_framework} NAMES ${_framework} REQUIRED)
    target_link_libraries(mfs_macos_source_context PRIVATE "${_mfs_${_framework}}")
endforeach()
# Instrumentation is a separate test executable, never the original oracle or
# migrated backend. Keep the exact fixed source as a separate translation unit.
add_executable(mfs_macos_observed_context source-context.mm
    "${_mfs_root}/Screencap/ScreenCaptureKitScreencap.mm")
get_target_property(_mfs_source_includes mfs_macos_source_context INCLUDE_DIRECTORIES)
get_target_property(_mfs_source_links mfs_macos_source_context LINK_LIBRARIES)
target_include_directories(mfs_macos_observed_context PRIVATE ${_mfs_source_includes})
target_link_libraries(mfs_macos_observed_context PRIVATE ${_mfs_source_links})
target_compile_definitions(mfs_macos_observed_context PRIVATE MFS_MAC_DISPATCH_OBSERVER=1)
target_compile_options(mfs_macos_observed_context PRIVATE -fno-objc-arc -fblocks -Wall -Wextra
    -include "${CMAKE_CURRENT_SOURCE_DIR}/dispatch-observer.hpp")
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/source-context-scope.txt"
    "Complete fixed ScreenCaptureKitScreencap header/Objective-C++ source compiles unchanged with manual reference counting. Screenshot base declaration is extracted unchanged; private namespace/logging exclude unrelated input/framework code. AppKit fixture lives on process main thread, original synchronous calls execute on joined callers, and native callbacks retain original five-second wait/conversion/release points. Controlled-window interior pixels and retained Mat owners are checked; resource/late-callback/format/full-display/manager parity and complete Rust acceptance remain incomplete. The explicit private adapter serves the initial Rust migration and exact selected manager-body scripts. Original manager forwarding uses real unchanged SCK; Input=None is stubbed, full manager/API/metadata acceptance remains pending. OpenCV stays outside product/default builds.\n")
