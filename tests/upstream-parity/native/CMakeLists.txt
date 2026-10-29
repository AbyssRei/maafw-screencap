cmake_minimum_required(VERSION 4.2)
project(mfs_p0_native LANGUAGES CXX)

if(NOT WIN32 OR NOT CMAKE_GENERATOR STREQUAL "Visual Studio 18 2026")
    message(FATAL_ERROR "This P0 reference currently requires Windows x64, VS2026 and MSBuild")
endif()
if(NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
    message(FATAL_ERROR "The initial reference target requires x64")
endif()

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_SCAN_FOR_MODULES OFF)
set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreadedDLL)
add_compile_options(/utf-8 /EHsc)
add_compile_definitions(NOMINMAX WIN32_LEAN_AND_MEAN)
set(OpenCV_STATIC ON)
set(OpenCV_ARCH x64)
set(OpenCV_RUNTIME vc18)
find_package(OpenCV REQUIRED COMPONENTS core imgproc imgcodecs)
option(MFS_BUILD_REFERENCE "Build the independent unpatched reference executable" ON)
option(MFS_BUILD_GDI_PARITY "Build the private GDI reference/conversion and window fixture" OFF)
option(MFS_BUILD_PRINT_PARITY "Include the original PrintWindow and owned rendering fixture" OFF)
option(MFS_BUILD_SCREEN_PARITY "Include original ScreenDC and window-helper observation" OFF)
option(MFS_BUILD_FRAMEPOOL_REFERENCE "Build the isolated original FramePool caller-context diagnostic" OFF)
option(MFS_BUILD_FRAMEPOOL_PARITY "Include the rendering/conversion adapter for private Rust FramePool parity" OFF)
option(MFS_BUILD_DESKTOP_DUP_REFERENCE "Build the isolated original DesktopDup/Window reference" OFF)
option(MFS_BUILD_MANAGER_PARITY "Build exact-source screenshot-manager scripts" OFF)
option(MFS_BUILD_RENDER_FIXTURE "Build independently owned native UI renderer" OFF)
option(MFS_BUILD_PIPEWIRE_CONTROL "Build the exact-source portable PipeWire consumer model" OFF)
option(MFS_BUILD_CUSTOM_PARITY "Build actual Custom callback/ImageBuffer acceptance" OFF)
if(MFS_BUILD_CUSTOM_PARITY)
    add_subdirectory("${CMAKE_CURRENT_SOURCE_DIR}/../custom-capture" custom-capture)
endif()
if(MFS_BUILD_PIPEWIRE_CONTROL)
    add_subdirectory("${CMAKE_CURRENT_SOURCE_DIR}/../pipewire-control" pipewire-control)
endif()
if(MFS_BUILD_MANAGER_PARITY AND NOT MFS_BUILD_GDI_PARITY)
    message(FATAL_ERROR "Manager parity requires the explicit common image adapter")
endif()
if(MFS_BUILD_RENDER_FIXTURE AND NOT MFS_BUILD_GDI_PARITY)
    message(FATAL_ERROR "The renderer requires the explicit common image adapter")
endif()
if(MFS_BUILD_FRAMEPOOL_PARITY AND (NOT MFS_BUILD_GDI_PARITY OR NOT MFS_BUILD_FRAMEPOOL_REFERENCE))
    message(FATAL_ERROR "FramePool parity requires the explicit common adapter and isolated original reference")
endif()
if(MFS_BUILD_SCREEN_PARITY AND NOT MFS_BUILD_GDI_PARITY)
    message(FATAL_ERROR "ScreenDC parity requires the selected common image adapter")
endif()
if(MFS_BUILD_PRINT_PARITY AND NOT MFS_BUILD_GDI_PARITY)
    message(FATAL_ERROR "PrintWindow parity uses the explicitly selected common image adapter")
endif()

add_library(mfs_p0_adapter SHARED interop.cpp)
target_link_libraries(mfs_p0_adapter PRIVATE ${OpenCV_LIBS})
target_include_directories(mfs_p0_adapter PRIVATE ${OpenCV_INCLUDE_DIRS})

if(MFS_BUILD_REFERENCE OR MFS_BUILD_GDI_PARITY OR MFS_BUILD_FRAMEPOOL_REFERENCE OR MFS_BUILD_DESKTOP_DUP_REFERENCE)
foreach(input MFS_UPSTREAM_ROOT MFS_MAAUTILS_ROOT MFS_BOOST_ROOT)
    if(NOT IS_DIRECTORY "${${input}}")
        message(FATAL_ERROR "Explicit external input ${input} is required")
    endif()
endforeach()
set(BOOST_INCLUDE_LIBRARIES regex uuid)
set(BOOST_BUILD_TESTING OFF)
set(BOOST_SKIP_INSTALL_RULES ON)
add_subdirectory("${MFS_BOOST_ROOT}" "${CMAKE_BINARY_DIR}/boost")

# Build only the utilities consumed by screenshot logging. The upstream utility
# project also configures OCR, inference and unrelated process dependencies.
# These source files are compiled without edits; this is a build adaptation.
add_library(mfs_reference_utils SHARED
    "${MFS_MAAUTILS_ROOT}/source/Logger/Logger.cpp"
    "${MFS_MAAUTILS_ROOT}/source/Platform/PlatformWin32.cpp"
    "${MFS_MAAUTILS_ROOT}/source/Encoding/Encoding.cpp"
    "${MFS_MAAUTILS_ROOT}/source/Uuid/Uuid.cpp")
target_compile_definitions(mfs_reference_utils PRIVATE MAA_UTILS_EXPORTS)
target_include_directories(mfs_reference_utils PUBLIC "${MFS_MAAUTILS_ROOT}/include" ${OpenCV_INCLUDE_DIRS})
target_link_libraries(mfs_reference_utils PUBLIC Boost::regex PRIVATE ${OpenCV_LIBS} Boost::uuid Psapi)
target_compile_definitions(mfs_reference_utils PRIVATE BOOST_ALL_NO_LIB)

if(MFS_BUILD_REFERENCE)
add_executable(mfs_reference_u
    reference.cpp
    "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/GdiScreencap.cpp")
target_include_directories(mfs_reference_u PRIVATE
    "${MFS_UPSTREAM_ROOT}/include"
    "${MFS_UPSTREAM_ROOT}/source/include"
    "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit")
target_link_libraries(mfs_reference_u PRIVATE mfs_reference_utils ${OpenCV_LIBS} Gdi32 User32)
endif()
if(MFS_BUILD_FRAMEPOOL_REFERENCE)
    # An executable isolates C++/WinRT's process factory/MTA state. This is a
    # source-only environment observation, not Rust FramePool parity acceptance.
    add_executable(mfs_framepool_reference framepool-reference.cpp print-parity.cpp print-thread.cpp pseudo-fixture.cpp
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/FramePoolScreencap.cpp"
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/PseudoMinimizeHelper.cpp")
    target_compile_definitions(mfs_framepool_reference PRIVATE MAA_FRAMEPOOL_SCREENCAP_AVAILABLE=1 MFS_FRAMEPOOL_PSEUDO_FIXTURE=1)
    target_include_directories(mfs_framepool_reference PRIVATE
        "${MFS_UPSTREAM_ROOT}/include"
        "${MFS_UPSTREAM_ROOT}/source/include"
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit"
        ${OpenCV_INCLUDE_DIRS})
    target_link_libraries(mfs_framepool_reference PRIVATE mfs_reference_utils ${OpenCV_LIBS}
        Gdi32 User32 Dwmapi D3D11 Dxgi WindowsApp RuntimeObject CoreMessaging)
    # Independent alpha/content causal diagnostic, sharing the exact native
    # source and fixture dependencies of the original caller-context entry.
    get_target_property(_framepool_sources mfs_framepool_reference SOURCES)
    list(REMOVE_ITEM _framepool_sources framepool-reference.cpp)
    add_executable(mfs_framepool_alpha_reference framepool-alpha-reference.cpp ${_framepool_sources}
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/PrintWindowScreencap.cpp")
    foreach(_property INCLUDE_DIRECTORIES COMPILE_DEFINITIONS LINK_LIBRARIES)
        get_target_property(_value mfs_framepool_reference ${_property})
        set_property(TARGET mfs_framepool_alpha_reference PROPERTY ${_property} "${_value}")
    endforeach()
endif()
if(MFS_BUILD_DESKTOP_DUP_REFERENCE)
    add_executable(mfs_desktop_dup_reference desktop-dup-reference.cpp print-parity.cpp print-thread.cpp
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/DesktopDupScreencap.cpp"
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/DesktopDupWindowScreencap.cpp")
    target_include_directories(mfs_desktop_dup_reference PRIVATE
        "${MFS_UPSTREAM_ROOT}/include" "${MFS_UPSTREAM_ROOT}/source/include"
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit" ${OpenCV_INCLUDE_DIRS})
    target_link_libraries(mfs_desktop_dup_reference PRIVATE mfs_reference_utils ${OpenCV_LIBS} Gdi32 User32 Dwmapi D3D11 Dxgi)
endif()
if(MFS_BUILD_GDI_PARITY)
add_library(mfs_gdi_parity SHARED
    gdi-parity.cpp
    gdi-probe.cpp
    gdi-probe-reference.cpp
    "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/GdiScreencap.cpp")
if(MFS_BUILD_MANAGER_PARITY)
    include("${CMAKE_CURRENT_SOURCE_DIR}/../../../build-support/manager-source.cmake")
    target_sources(mfs_gdi_parity PRIVATE manager-script.cpp manager-gdi-reference.cpp)
    target_include_directories(mfs_gdi_parity PRIVATE "${CMAKE_CURRENT_BINARY_DIR}")
endif()
target_include_directories(mfs_gdi_parity PRIVATE
    "${MFS_UPSTREAM_ROOT}/include"
    "${MFS_UPSTREAM_ROOT}/source/include"
    "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit"
    ${OpenCV_INCLUDE_DIRS})
target_link_libraries(mfs_gdi_parity PRIVATE mfs_reference_utils ${OpenCV_LIBS} Gdi32 User32)
file(TO_CMAKE_PATH "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/GdiScreencap.cpp" MFS_GDI_CPP_PATH)
target_compile_definitions(mfs_gdi_parity PRIVATE MFS_UPSTREAM_GDI_CPP="${MFS_GDI_CPP_PATH}")
if((MFS_BUILD_FRAMEPOOL_PARITY OR MFS_BUILD_DESKTOP_DUP_REFERENCE OR MFS_BUILD_SCREEN_PARITY OR MFS_BUILD_RENDER_FIXTURE) AND NOT MFS_BUILD_PRINT_PARITY)
    target_sources(mfs_gdi_parity PRIVATE print-parity.cpp print-thread.cpp)
    target_link_libraries(mfs_gdi_parity PRIVATE Dwmapi)
endif()
if(MFS_BUILD_FRAMEPOOL_PARITY AND NOT MFS_BUILD_PRINT_PARITY)
    target_sources(mfs_gdi_parity PRIVATE pseudo-fixture.cpp pseudo-script.cpp)
    target_compile_definitions(mfs_gdi_parity PRIVATE MFS_FRAMEPOOL_PSEUDO_FIXTURE=1)
    file(TO_CMAKE_PATH "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/PseudoMinimizeHelper.cpp" MFS_PSEUDO_CPP_PATH)
    target_compile_definitions(mfs_gdi_parity PRIVATE MFS_UPSTREAM_PSEUDO_CPP="${MFS_PSEUDO_CPP_PATH}")
endif()
if(MFS_BUILD_SCREEN_PARITY)
target_sources(mfs_gdi_parity PRIVATE window-script.cpp screen-parity.cpp screen-probe-reference.cpp
    "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/ScreenDCScreencap.cpp")
target_link_libraries(mfs_gdi_parity PRIVATE Dwmapi)
file(TO_CMAKE_PATH "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/ScreenDCScreencap.cpp" MFS_SCREEN_CPP_PATH)
target_compile_definitions(mfs_gdi_parity PRIVATE MFS_UPSTREAM_SCREEN_CPP="${MFS_SCREEN_CPP_PATH}")
endif()
if(MFS_BUILD_PRINT_PARITY)
target_sources(mfs_gdi_parity PRIVATE print-parity.cpp print-read-probe.cpp print-query-probe.cpp
    print-conversion-probe.cpp print-fault-reference.cpp pseudo-script.cpp print-pseudo.cpp pseudo-fixture.cpp print-thread.cpp
    "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/PseudoMinimizeHelper.cpp"
    "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/PrintWindowScreencap.cpp")
target_link_libraries(mfs_gdi_parity PRIVATE Dwmapi)
file(TO_CMAKE_PATH "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/PrintWindowScreencap.cpp" MFS_PRINT_CPP_PATH)
target_compile_definitions(mfs_gdi_parity PRIVATE MFS_UPSTREAM_PRINT_CPP="${MFS_PRINT_CPP_PATH}")
file(TO_CMAKE_PATH "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/PseudoMinimizeHelper.cpp" MFS_PSEUDO_CPP_PATH)
target_compile_definitions(mfs_gdi_parity PRIVATE MFS_UPSTREAM_PSEUDO_CPP="${MFS_PSEUDO_CPP_PATH}")
endif()
endif()
endif()

file(GENERATE OUTPUT "${CMAKE_BINARY_DIR}/toolchain-$<CONFIG>.txt" CONTENT
    "generator=${CMAKE_GENERATOR}\ninstance=${CMAKE_GENERATOR_INSTANCE}\nplatform=${CMAKE_VS_PLATFORM_NAME}\ncompiler=${CMAKE_CXX_COMPILER}\ncompiler_version=${CMAKE_CXX_COMPILER_VERSION}\nsdk=${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION}\nruntime=${CMAKE_MSVC_RUNTIME_LIBRARY}\n")
