cmake_minimum_required(VERSION 4.2)
project(mfs_wlr_source_context LANGUAGES C CXX)
if(NOT CMAKE_SYSTEM_NAME STREQUAL "Linux" OR CMAKE_CROSSCOMPILING OR
   NOT CMAKE_SIZEOF_VOID_P EQUAL 8 OR NOT CMAKE_SYSTEM_PROCESSOR MATCHES "^(x86_64|amd64|AMD64)$")
    message(FATAL_ERROR "This source context requires native Linux x64")
endif()
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_SCAN_FOR_MODULES OFF)
set(OpenCV_STATIC ON)
find_package(OpenCV REQUIRED COMPONENTS core imgproc)
find_package(PkgConfig REQUIRED)
find_package(Threads REQUIRED)
pkg_check_modules(WAYLAND REQUIRED IMPORTED_TARGET wayland-client)
pkg_check_modules(WAYLAND_SERVER REQUIRED IMPORTED_TARGET wayland-server)
if(NOT WAYLAND_VERSION STREQUAL MFS_WAYLAND_VERSION)
    message(FATAL_ERROR "Reference selected a different libwayland version")
endif()
if(NOT WAYLAND_SERVER_VERSION STREQUAL MFS_WAYLAND_VERSION OR
   NOT IS_DIRECTORY "${MFS_WLROOTS_PROTOCOL_ROOT}")
    message(FATAL_ERROR "Explicit matching native server and prepared protocol XML are required")
endif()
find_program(MFS_WAYLAND_SCANNER wayland-scanner REQUIRED)
foreach(_protocol "screencopy;wlr-screencopy-unstable-v1" "pointer;wlr-virtual-pointer-unstable-v1" "keyboard;virtual-keyboard-unstable-v1")
    list(GET _protocol 0 _name)
    list(GET _protocol 1 _file)
    set(_xml "${MFS_WLROOTS_PROTOCOL_ROOT}/${_file}.xml")
    add_custom_command(OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/${_name}-server.h" "${CMAKE_CURRENT_BINARY_DIR}/${_name}-server.c"
        COMMAND "${MFS_WAYLAND_SCANNER}" server-header "${_xml}" "${CMAKE_CURRENT_BINARY_DIR}/${_name}-server.h"
        COMMAND "${MFS_WAYLAND_SCANNER}" private-code "${_xml}" "${CMAKE_CURRENT_BINARY_DIR}/${_name}-server.c"
        DEPENDS "${_xml}" VERBATIM)
    list(APPEND _server_protocols "${CMAKE_CURRENT_BINARY_DIR}/${_name}-server.c" "${CMAKE_CURRENT_BINARY_DIR}/${_name}-server.h")
endforeach()
add_executable(mfs_wlr_protocol_server protocol-server.c ${_server_protocols})
target_include_directories(mfs_wlr_protocol_server PRIVATE "${CMAKE_CURRENT_BINARY_DIR}")
target_compile_features(mfs_wlr_protocol_server PRIVATE c_std_11)
target_compile_definitions(mfs_wlr_protocol_server PRIVATE _GNU_SOURCE)
target_compile_options(mfs_wlr_protocol_server PRIVATE -Wall -Wextra -Werror)
target_link_libraries(mfs_wlr_protocol_server PRIVATE PkgConfig::WAYLAND_SERVER ${CMAKE_DL_LIBS})
include("${CMAKE_CURRENT_SOURCE_DIR}/../../../build-support/pipewire-base-source.cmake")
include("${CMAKE_CURRENT_SOURCE_DIR}/../../../build-support/linux-manager-source.cmake")
set(_mfs_root "${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit")
add_executable(mfs_wlr_source_context source-context.cpp
    "${_mfs_root}/Screencap/WlrScreencap.cpp"
    "${_mfs_root}/Wayland/WaylandClient.cpp"
    "${_mfs_root}/Wayland/WaylandHelper.cpp"
    "${_mfs_root}/Base/MemfdBuffer.cpp"
    "${_mfs_root}/Wayland/Protocol/wlr-screencopy-priv.c"
    "${_mfs_root}/Wayland/Protocol/wlr-virtual-pointer-priv.c"
    "${_mfs_root}/Wayland/Protocol/virtual-keyboard-priv.c")
target_compile_definitions(mfs_wlr_source_context PRIVATE _GNU_SOURCE)
target_include_directories(mfs_wlr_source_context PRIVATE
    "${CMAKE_CURRENT_BINARY_DIR}/overlay" "${CMAKE_CURRENT_SOURCE_DIR}/overlay"
    "${CMAKE_CURRENT_BINARY_DIR}" "${MFS_UPSTREAM_ROOT}/include" "${_mfs_root}" ${OpenCV_INCLUDE_DIRS})
target_link_libraries(mfs_wlr_source_context PRIVATE
    PkgConfig::WAYLAND Threads::Threads ${OpenCV_LIBS} ${CMAKE_DL_LIBS})
# The wrapper only observes calls made by the compiled source and forwards each
# one immediately, unchanged, to the actual system libwayland entry point.
target_link_options(mfs_wlr_source_context PRIVATE "LINKER:--wrap=wl_display_roundtrip")
# Reuse the self-owned, header-checked conversion acceptance source. It includes
# OpenCV only and introduces no PipeWire runtime or capture dependency.
add_library(mfs_wlr_opencv SHARED ../linux-pipewire/conversion-adapter.cpp)
target_include_directories(mfs_wlr_opencv PRIVATE ${OpenCV_INCLUDE_DIRS})
target_link_libraries(mfs_wlr_opencv PRIVATE Threads::Threads ${OpenCV_LIBS} ${CMAKE_DL_LIBS})
target_link_options(mfs_wlr_opencv PRIVATE "LINKER:--exclude-libs,ALL")
set_target_properties(mfs_wlr_opencv PROPERTIES CXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN YES)
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/source-context-scope.txt"
    "Complete fixed WlrScreencap, WaylandClient, MemfdBuffer, WaylandHelper and original generated C protocol sources compile unchanged. Only the screenshot base is extracted. Private namespace/logging and a kernel-random external UUID name provider replace unrelated framework dependencies. Passive virtual keyboard/pointer managers are required by the original client; no input action runs. A forwarding linker wrapper records actual source roundtrip recursion. Native capture, buffers, C++ owners and OpenCV conversion remain real. This is original source context only, not a Rust backend, original UUID implementation or full Wlr/P1 acceptance.\n")
