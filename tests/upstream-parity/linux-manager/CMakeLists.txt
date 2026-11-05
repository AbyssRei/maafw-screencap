cmake_minimum_required(VERSION 4.2)
project(mfs_linux_manager_model LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_SCAN_FOR_MODULES OFF)
if(WIN32)
    if(NOT CMAKE_GENERATOR STREQUAL "Visual Studio 18 2026" OR NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
        message(FATAL_ERROR "The Windows model requires VS2026/MSBuild x64")
    endif()
    set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreadedDLL)
    set(OpenCV_ARCH x64)
    set(OpenCV_RUNTIME vc18)
endif()
set(OpenCV_STATIC ON)
find_package(OpenCV REQUIRED COMPONENTS core)
include("${CMAKE_CURRENT_SOURCE_DIR}/../../../build-support/linux-manager-source.cmake")
include("${CMAKE_CURRENT_SOURCE_DIR}/../../../build-support/pipewire-base-source.cmake")
add_executable(mfs_linux_none_reference none-reference.cpp)
target_include_directories(mfs_linux_none_reference PRIVATE
    "${CMAKE_CURRENT_BINARY_DIR}/overlay" "${CMAKE_CURRENT_SOURCE_DIR}/none-overlay"
    "${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit" ${OpenCV_INCLUDE_DIRS})
target_link_libraries(mfs_linux_none_reference PRIVATE ${OpenCV_LIBS})
if(MSVC)
    target_compile_options(mfs_linux_none_reference PRIVATE /utf-8 /EHsc)
endif()
add_executable(mfs_linux_manager_model model.cpp)
target_include_directories(mfs_linux_manager_model PRIVATE "${CMAKE_CURRENT_BINARY_DIR}" "${MFS_UPSTREAM_ROOT}/include" ${OpenCV_INCLUDE_DIRS})
target_link_libraries(mfs_linux_manager_model PRIVATE ${OpenCV_LIBS})
if(MSVC)
    target_compile_options(mfs_linux_manager_model PRIVATE /utf-8 /EHsc)
endif()
if(DEFINED MFS_MEOJSON_INCLUDE)
    if(NOT EXISTS "${MFS_MEOJSON_INCLUDE}/meojson/json.hpp")
        message(FATAL_ERROR "Configuration oracle requires its explicitly prepared latest formal meojson include prefix")
    endif()
    include("${CMAKE_CURRENT_SOURCE_DIR}/../../../build-support/linux-config-source.cmake")
    add_subdirectory("${CMAKE_CURRENT_SOURCE_DIR}/../manager-metadata" metadata)
    add_executable(mfs_linux_config_model config-model.cpp
        "${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit/Compat/WlRootsControlUnitCompat.cpp"
        "${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit/Compat/KWinControlUnitCompat.cpp")
    target_compile_definitions(mfs_linux_config_model PRIVATE MAA_CONTROL_UNIT_EXPORTS MAA_TOOLKIT_EXPORTS MAA_VERSION="mfs-p1-config-oracle")
    target_include_directories(mfs_linux_config_model PRIVATE "${CMAKE_CURRENT_BINARY_DIR}/config-overlay"
        "${CMAKE_CURRENT_SOURCE_DIR}/config-overlay" "${CMAKE_CURRENT_BINARY_DIR}" "${MFS_MEOJSON_INCLUDE}"
        "${MFS_UPSTREAM_ROOT}/include" ${OpenCV_INCLUDE_DIRS})
    target_link_libraries(mfs_linux_config_model PRIVATE ${OpenCV_LIBS})
    if(MSVC)
        # Original extern-C creation can throw into this C++ observer. Do not
        # assume those functions are nothrow via /EHc.
        target_compile_options(mfs_linux_config_model PRIVATE /utf-8 /EHs)
    endif()
    # Full original manager/API bodies, independently built beside the older
    # extracted-body oracle. This query-only caller never connects a backend.
    set(_whole_overlay "${CMAKE_CURRENT_BINARY_DIR}/whole-query-overlay")
    foreach(_name "Input/EiInput.h" "Input/UInput.h" "Input/WlrInput.h" "Input/VkToEvdev.h"
        "Screencap/PipeWireScreencap.h" "Screencap/WlrScreencap.h")
        get_filename_component(_parent "${_whole_overlay}/${_name}" DIRECTORY)
        file(MAKE_DIRECTORY "${_parent}")
        file(WRITE "${_whole_overlay}/${_name}" "#pragma once\n// Query-only rejecting type is declared by whole-query-boundary.hpp.\n")
    endforeach()
    add_executable(mfs_linux_whole_config_model config-model.cpp
        "${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit/API/LinuxControlUnitAPI.cpp"
        "${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit/Input/NoneInput.cpp"
        "${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit/Compat/WlRootsControlUnitCompat.cpp"
        "${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit/Compat/KWinControlUnitCompat.cpp")
    target_compile_definitions(mfs_linux_whole_config_model PRIVATE MFS_WHOLE_LINUX_QUERY
        MFS_WHOLE_LINUX_MANAGER_HEADER="${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit/Manager/LinuxControlUnitMgr.h"
        MFS_WHOLE_LINUX_MANAGER_CPP="${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit/Manager/LinuxControlUnitMgr.cpp"
        MAA_CONTROL_UNIT_EXPORTS MAA_TOOLKIT_EXPORTS MAA_VERSION="mfs-p1-config-oracle")
    target_include_directories(mfs_linux_whole_config_model PRIVATE "${_whole_overlay}"
        "${CMAKE_CURRENT_BINARY_DIR}/config-overlay" "${CMAKE_CURRENT_SOURCE_DIR}/config-overlay"
        "${CMAKE_CURRENT_BINARY_DIR}" "${MFS_MEOJSON_INCLUDE}" "${MFS_UPSTREAM_ROOT}/include"
        "${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit" ${OpenCV_INCLUDE_DIRS})
    target_link_libraries(mfs_linux_whole_config_model PRIVATE ${OpenCV_LIBS})
    if(MSVC)
        target_compile_options(mfs_linux_whole_config_model PRIVATE /utf-8 /EHs)
    endif()
    file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/whole-query-scope.txt"
        "Whole unchanged F LinuxControlUnitMgr.cpp/header and LinuxControlUnitAPI.cpp plus real NoneInput.cpp; actual virtual query/caller Mat consumption only, no connect/input action/native capture. Explicit rejecting Wlr/PipeWire/input/key-translation boundaries compile unselected dependencies only. This closes whole manager source/vtable query compilation, not actual native manager capture, original input construction or P1.\n")
endif()
