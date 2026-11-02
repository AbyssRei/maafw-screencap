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
option(MFS_BUILD_REFERENCE "Build the independent selected-source reference executable" ON)
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
    add_executable(mfs_framepool_target_loss_reference framepool-target-loss-reference.cpp ${_framepool_sources})
    # Source-only order observation; no manager or native source is rewritten.
    add_executable(mfs_framepool_order_reference framepool-order-reference.cpp ${_framepool_sources}
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/DesktopDupScreencap.cpp"
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/DesktopDupWindowScreencap.cpp")
    add_executable(mfs_framepool_border_reference framepool-border-reference.cpp ${_framepool_sources}
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/DesktopDupScreencap.cpp"
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/DesktopDupWindowScreencap.cpp")
    target_include_directories(mfs_framepool_border_reference PRIVATE "${CMAKE_CURRENT_BINARY_DIR}")
    # Isolate the exact original device/swapchain operation from item/session
    # creation. This fragment is a separate SDK control, not a rewritten backend.
    file(READ "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/FramePoolScreencap.cpp" _framepool_device_source)
    string(REPLACE "\r\n" "\n" _framepool_device_source "${_framepool_device_source}")
    string(FIND "${_framepool_device_source}" "    DXGI_SWAP_CHAIN_DESC swap_chain_desc = { };" _framepool_device_begin)
    if(_framepool_device_begin LESS 0)
        message(FATAL_ERROR "Missing exact FramePool source device descriptor")
    endif()
    string(SUBSTRING "${_framepool_device_source}" ${_framepool_device_begin} -1 _framepool_device_tail)
    string(FIND "${_framepool_device_tail}" "    if (FAILED(ret))" _framepool_device_end)
    if(_framepool_device_end LESS_EQUAL 0)
        message(FATAL_ERROR "Missing exact FramePool source device call end")
    endif()
    string(SUBSTRING "${_framepool_device_tail}" 0 ${_framepool_device_end} _framepool_device_only)
    string(REGEX MATCHALL "D3D11CreateDeviceAndSwapChain" _framepool_device_calls "${_framepool_device_only}")
    list(LENGTH _framepool_device_calls _framepool_device_count)
    if(NOT _framepool_device_count EQUAL 1)
        message(FATAL_ERROR "Expected exactly one original device/swapchain call")
    endif()
    file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/framepool-device-only.inc" "${_framepool_device_only}")
    set(_framepool_query_sources ${_framepool_sources})
    list(REMOVE_ITEM _framepool_query_sources "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/FramePoolScreencap.cpp")
    add_executable(mfs_framepool_query_reference framepool-query-reference.cpp ${_framepool_query_sources})
    file(TO_CMAKE_PATH "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/FramePoolScreencap.cpp" MFS_FRAMEPOOL_CPP_PATH)
    # Independent forwarding observer. These three exact expressions are the
    # only edits; native acquisition/Close, sleeps, clocks and catches stay put.
    file(READ "${MFS_FRAMEPOOL_CPP_PATH}" _framepool_poll_source)
    set(_poll_site_0 "auto old_frame = cap_frame_pool_.TryGetNextFrame()")
    set(_poll_site_1 "frame = cap_frame_pool_.TryGetNextFrame();")
    set(_poll_site_2 "return cached_image_.clone();")
    set(_poll_replacement_0 "auto old_frame = ::mfs_poll_probe::next(cap_frame_pool_, true)")
    set(_poll_replacement_1 "frame = ::mfs_poll_probe::next(cap_frame_pool_, false);")
    set(_poll_replacement_2 "return ::mfs_poll_probe::clone(cached_image_);")
    foreach(_index RANGE 0 2)
        set(_site "${_poll_site_${_index}}")
        set(_replacement "${_poll_replacement_${_index}}")
        string(LENGTH "${_framepool_poll_source}" _before_length)
        string(LENGTH "${_site}" _site_length)
        string(REPLACE "${_site}" "" _without_site "${_framepool_poll_source}")
        string(LENGTH "${_without_site}" _after_length)
        math(EXPR _difference "${_before_length}-${_after_length}")
        if(NOT _difference EQUAL _site_length)
            message(FATAL_ERROR "Expected one exact original FramePool polling/cache expression")
        endif()
        string(REPLACE "${_site}" "${_replacement}" _framepool_poll_source "${_framepool_poll_source}")
    endforeach()
    set(_framepool_poll_path "${CMAKE_CURRENT_BINARY_DIR}/framepool-poll-observed.cpp")
    file(WRITE "${_framepool_poll_path}" "#include \"framepool-poll-probe.hpp\"\n${_framepool_poll_source}")
    add_executable(mfs_framepool_alpha_poll_reference EXCLUDE_FROM_ALL framepool-alpha-reference.cpp ${_framepool_query_sources}
        "${_framepool_poll_path}"
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/PrintWindowScreencap.cpp")
    target_compile_definitions(mfs_framepool_alpha_poll_reference PRIVATE MFS_FRAMEPOOL_POLL_OBSERVER=1)
    target_include_directories(mfs_framepool_alpha_poll_reference PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}"
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap")
    target_compile_definitions(mfs_framepool_query_reference PRIVATE MFS_UPSTREAM_FRAMEPOOL_CPP="${MFS_FRAMEPOOL_CPP_PATH}")
    add_executable(mfs_framepool_map_reference framepool-query-reference.cpp ${_framepool_query_sources})
    target_compile_definitions(mfs_framepool_map_reference PRIVATE MFS_UPSTREAM_FRAMEPOOL_CPP="${MFS_FRAMEPOOL_CPP_PATH}" MFS_FRAMEPOOL_MAP_OBSERVER=1)
    # Controlled query results over the real item, preserving all four original
    # call sites/catches. Generated only in the native acceptance build tree.
    file(READ "${MFS_FRAMEPOOL_CPP_PATH}" _framepool_size_source)
    string(REGEX MATCHALL "cap_item_[.]Size[(][)]" _framepool_size_sites "${_framepool_size_source}")
    list(LENGTH _framepool_size_sites _framepool_size_count)
    if(NOT _framepool_size_count EQUAL 4)
        message(FATAL_ERROR "Expected four exact FramePool item Size call sites")
    endif()
    string(REPLACE "cap_item_.Size()" "::mfs_size_probe::size(cap_item_)" _framepool_size_source "${_framepool_size_source}")
    set(_framepool_size_path "${CMAKE_CURRENT_BINARY_DIR}/framepool-size-observed.cpp")
    file(WRITE "${_framepool_size_path}" "${_framepool_size_source}")
    add_executable(mfs_framepool_size_reference framepool-query-reference.cpp ${_framepool_query_sources})
    target_compile_definitions(mfs_framepool_size_reference PRIVATE MFS_UPSTREAM_FRAMEPOOL_CPP="${_framepool_size_path}" MFS_FRAMEPOOL_SIZE_OBSERVER=1)
    target_include_directories(mfs_framepool_size_reference PRIVATE "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap")
    # Separate diagnostic: one readonly frame.ContentSize observation is inserted
    # before Surface. The unchanged original target above remains authoritative.
    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${MFS_FRAMEPOOL_CPP_PATH}")
    file(READ "${MFS_FRAMEPOOL_CPP_PATH}" _framepool_observed_source)
    string(REGEX MATCHALL "auto surface = frame[.]Surface[(][)]" _framepool_surface_sites "${_framepool_observed_source}")
    list(LENGTH _framepool_surface_sites _framepool_surface_count)
    if(NOT _framepool_surface_count EQUAL 1)
        message(FATAL_ERROR "Expected one exact FramePool Surface observation site")
    endif()
    string(REPLACE "auto surface = frame.Surface();"
        "mfs_observe_frame_content(frame.ContentSize());\n    auto surface = frame.Surface();"
        _framepool_observed_source "${_framepool_observed_source}")
    set(_framepool_observed_path "${CMAKE_CURRENT_BINARY_DIR}/framepool-content-observed.cpp")
    file(WRITE "${_framepool_observed_path}" "${_framepool_observed_source}")
    add_executable(mfs_framepool_content_observed framepool-query-reference.cpp ${_framepool_query_sources})
    target_compile_definitions(mfs_framepool_content_observed PRIVATE
        MFS_UPSTREAM_FRAMEPOOL_CPP="${_framepool_observed_path}" MFS_FRAMEPOOL_CONTENT_OBSERVER=1)
    target_include_directories(mfs_framepool_content_observed PRIVATE "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap")
    foreach(_property INCLUDE_DIRECTORIES COMPILE_DEFINITIONS LINK_LIBRARIES)
        get_target_property(_value mfs_framepool_reference ${_property})
        set_property(TARGET mfs_framepool_alpha_reference PROPERTY ${_property} "${_value}")
        set_property(TARGET mfs_framepool_alpha_poll_reference APPEND PROPERTY ${_property} "${_value}")
        set_property(TARGET mfs_framepool_target_loss_reference PROPERTY ${_property} "${_value}")
        set_property(TARGET mfs_framepool_order_reference PROPERTY ${_property} "${_value}")
        set_property(TARGET mfs_framepool_border_reference APPEND PROPERTY ${_property} "${_value}")
        set_property(TARGET mfs_framepool_query_reference APPEND PROPERTY ${_property} "${_value}")
        set_property(TARGET mfs_framepool_map_reference APPEND PROPERTY ${_property} "${_value}")
        set_property(TARGET mfs_framepool_size_reference APPEND PROPERTY ${_property} "${_value}")
        set_property(TARGET mfs_framepool_content_observed APPEND PROPERTY ${_property} "${_value}")
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
    add_executable(mfs_desktop_map_reference desktop-map-reference.cpp print-parity.cpp print-thread.cpp)
    target_include_directories(mfs_desktop_map_reference PRIVATE
        "${MFS_UPSTREAM_ROOT}/include" "${MFS_UPSTREAM_ROOT}/source/include"
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit" ${OpenCV_INCLUDE_DIRS})
    file(TO_CMAKE_PATH "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/DesktopDupScreencap.cpp" MFS_DESKTOP_DUP_CPP_PATH)
    target_compile_definitions(mfs_desktop_map_reference PRIVATE MFS_UPSTREAM_DESKTOP_DUP_CPP="${MFS_DESKTOP_DUP_CPP_PATH}")
    target_link_libraries(mfs_desktop_map_reference PRIVATE mfs_reference_utils ${OpenCV_LIBS} Gdi32 User32 Dwmapi D3D11 Dxgi)
endif()
if(MFS_BUILD_GDI_PARITY)
add_executable(mfs_render_marker_control print-marker-control.cpp)
target_link_libraries(mfs_render_marker_control PRIVATE Gdi32 User32 Dwmapi)
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
