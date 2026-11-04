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
add_subdirectory("${CMAKE_CURRENT_SOURCE_DIR}/../macos-manager-queries" manager-queries)
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
add_library(mfs_macos_opencv SHARED ../linux-pipewire/conversion-adapter.cpp acceptance-evidence.cpp manager-script.cpp
    sample-fixture.mm "${_mfs_root}/Screencap/ScreenCaptureKitScreencap.mm")
target_include_directories(mfs_macos_opencv PRIVATE ${OpenCV_INCLUDE_DIRS}
    "${CMAKE_CURRENT_BINARY_DIR}" "${CMAKE_CURRENT_BINARY_DIR}/overlay"
    "${CMAKE_CURRENT_SOURCE_DIR}/overlay" "${MFS_UPSTREAM_ROOT}/include" "${_mfs_root}")
target_compile_options(mfs_macos_opencv PRIVATE
    "$<$<COMPILE_LANGUAGE:OBJCXX>:-fno-objc-arc>" "$<$<COMPILE_LANGUAGE:OBJCXX>:-fblocks>" -Wall -Wextra)
target_link_libraries(mfs_macos_opencv PRIVATE Threads::Threads ${OpenCV_LIBS} "${_mfs_proc}")
set_target_properties(mfs_macos_opencv PROPERTIES CXX_VISIBILITY_PRESET hidden OBJCXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN YES)
foreach(_framework AppKit ApplicationServices CoreFoundation CoreMedia CoreVideo ScreenCaptureKit)
    find_library(_mfs_${_framework} NAMES ${_framework} REQUIRED)
    target_link_libraries(mfs_macos_source_context PRIVATE "${_mfs_${_framework}}")
    target_link_libraries(mfs_macos_opencv PRIVATE "${_mfs_${_framework}}")
endforeach()
# Instrumentation is a separate test executable, never the original oracle or
# migrated backend. Keep the exact fixed source as a separate translation unit.
add_executable(mfs_macos_observed_context source-context.mm capture-observer.mm
    "${_mfs_root}/Screencap/ScreenCaptureKitScreencap.mm")
get_target_property(_mfs_source_includes mfs_macos_source_context INCLUDE_DIRECTORIES)
get_target_property(_mfs_source_links mfs_macos_source_context LINK_LIBRARIES)
target_include_directories(mfs_macos_observed_context PRIVATE ${_mfs_source_includes})
target_link_libraries(mfs_macos_observed_context PRIVATE ${_mfs_source_links})
target_compile_definitions(mfs_macos_observed_context PRIVATE MFS_MAC_DISPATCH_OBSERVER=1)
target_compile_options(mfs_macos_observed_context PRIVATE -fno-objc-arc -fblocks -Wall -Wextra
    -include "${CMAKE_CURRENT_SOURCE_DIR}/dispatch-observer.hpp")
# Loaded only by the explicitly selected Rust scene-child entry, on main.
# Keep it separate from the existing adapter so old consumer processes never
# register these extra fixture classes merely by loading their converter.
add_library(mfs_macos_same_binary_scene SHARED same-binary-scene.mm same-binary-original.mm
    "${_mfs_root}/Screencap/ScreenCaptureKitScreencap.mm")
target_include_directories(mfs_macos_same_binary_scene PRIVATE ${_mfs_source_includes})
target_link_libraries(mfs_macos_same_binary_scene PRIVATE ${_mfs_source_links})
target_compile_options(mfs_macos_same_binary_scene PRIVATE -fno-objc-arc -fblocks -Wall -Wextra)
set_target_properties(mfs_macos_same_binary_scene PROPERTIES
    CXX_VISIBILITY_PRESET hidden OBJCXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN YES)
# Only this separate class object receives selector aliases. The fixture scenes,
# ordinary reference targets and Rust capture calls keep their original selectors.
add_library(mfs_macos_native_sample_class OBJECT "${_mfs_root}/Screencap/ScreenCaptureKitScreencap.mm")
add_executable(mfs_mac_live_sample_hook_control live-sample-hook-control.cpp)
add_executable(mfs_mac_callback_ledger_control callback-ledger-control.cpp)
target_compile_options(mfs_mac_callback_ledger_control PRIVATE -Wall -Wextra -Werror)
target_link_libraries(mfs_mac_callback_ledger_control PRIVATE Threads::Threads)
target_compile_options(mfs_mac_live_sample_hook_control PRIVATE -Wall -Wextra -Werror)
target_link_libraries(mfs_mac_live_sample_hook_control PRIVATE Threads::Threads)
target_include_directories(mfs_macos_native_sample_class PRIVATE ${_mfs_source_includes})
target_compile_definitions(mfs_macos_native_sample_class PRIVATE MFS_MAC_NATIVE_SAMPLE_ALIAS=1)
target_compile_options(mfs_macos_native_sample_class PRIVATE -fno-objc-arc -fblocks -Wall -Wextra
    -include "${CMAKE_CURRENT_SOURCE_DIR}/native-sample-observer.hpp")
set_target_properties(mfs_macos_native_sample_class PROPERTIES POSITION_INDEPENDENT_CODE ON
    CXX_VISIBILITY_PRESET hidden OBJCXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN YES)
add_library(mfs_macos_native_sample_scene SHARED same-binary-scene.mm same-binary-original.mm
    native-sample-observer.mm $<TARGET_OBJECTS:mfs_macos_native_sample_class>)
target_include_directories(mfs_macos_native_sample_scene PRIVATE ${_mfs_source_includes})
target_link_libraries(mfs_macos_native_sample_scene PRIVATE ${_mfs_source_links})
target_compile_definitions(mfs_macos_native_sample_scene PRIVATE MFS_MAC_NATIVE_SAMPLE_OBSERVER=1)
target_compile_options(mfs_macos_native_sample_scene PRIVATE -fno-objc-arc -fblocks -Wall -Wextra)
set_target_properties(mfs_macos_native_sample_scene PROPERTIES
    CXX_VISIBILITY_PRESET hidden OBJCXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN YES)
add_executable(mfs_macos_sample_association_control native-sample-state-control.cpp native-sample-state-control-helper.cpp)
target_compile_options(mfs_macos_sample_association_control PRIVATE -Wall -Wextra -Werror)
target_link_libraries(mfs_macos_sample_association_control PRIVATE Threads::Threads)
set_target_properties(mfs_macos_sample_association_control PROPERTIES
    CXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN YES)
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/source-context-scope.txt"
    "Complete fixed ScreenCaptureKitScreencap header/Objective-C++ source compiles unchanged with manual reference counting. Screenshot base declaration is extracted unchanged; private namespace/logging exclude unrelated input/framework code. AppKit fixture lives on process main thread, original synchronous calls execute on joined callers, and native callbacks retain original five-second wait/conversion/release points. Controlled-window interior pixels and retained Mat owners are checked; resource/late-callback/format/full-display/manager parity and complete Rust acceptance remain incomplete. The explicit private adapter serves the initial Rust migration and exact selected manager-body scripts. It also compiles the same unchanged source separately for its image_from_sample_buffer helper; native-owned CVPixelBuffer/ready CMSampleBuffer fixtures exercise actual padded strides, conversion, overwrite and retained CPU owners without capturing a desktop or requesting capture permission. Original manager forwarding uses real unchanged SCK; Input=None is stubbed, full manager/API/metadata acceptance remains pending. OpenCV stays outside product/default builds.\n")
file(APPEND "${CMAKE_CURRENT_BINARY_DIR}/source-context-scope.txt"
    "Separate display-scene-cg producer uses CoreGraphics active-display/window metadata and the same AppKit Window/presentation/stop loop. It requires one active display and records two completed geometry queries plus actual owner/closure. The reviewed producer mode makes no explicit ScreenCaptureKit query/capture call; this is not framework-internal call observation. Source consumer still uses its unchanged queried display-patch and five-second source wait. Same executable path/hash/mode and full paired payload/owner checks remain mandatory. Old scenes/oracles are retained, no retry/substitution or root-cause/P1 claim.\n")
file(APPEND "${CMAKE_CURRENT_BINARY_DIR}/source-context-scope.txt"
    "A separate full-display RGB scene uses only CG producer geometry, requires one 1:1 display and exact full-window bounds, and leaves old scenes/oracles unchanged. Its dedicated Rust test records original-first/Rust-first synchronous call intervals and all full-target/held CPU bytes. Twelve known images across both order arms are required by its separate validator; static content does not prove changing-display freshness, general scaling, callback drainage, old failed oracle recovery or full P1 acceptance. Actual native compilation/capture must be established by that batch's checks, never by configuration alone.\n")
file(APPEND "${CMAKE_CURRENT_BINARY_DIR}/source-context-scope.txt"
    "A further separate view/census scene serves seven explicit main-thread witness requests around the six capture calls. NSView cacheDisplay draws only the owned view into CPU storage. An empty factory-compatible bitmap is explicitly tagged sRGB and initialized before drawing; getPixel reads validated eight-bit RGBA components into top-left BGRA8. CGWindowListCopyWindowInfo records actual window identity/order/layer/bounds/names, the OnScreenOnly query and an optional visibility flag without inventing its value. Interleaving these observations can affect timing and does not observe simultaneous compositor pixels or prove a dot owner. Native strict full-RGB assertions remain; the separate records validator may verify complete diagnostic data while preserving those native pixel failures. No pixels are masked or substituted and original F capture code is unchanged.\n")
file(APPEND "${CMAKE_CURRENT_BINARY_DIR}/source-context-scope.txt"
    "An additional independent maximum-level owned-window fixture queries public CGWindowLevelForKey ScreenSaver/Maximum values, sets only its own NSWindow, and requires requested/getter/census levels and actual owner/closure to agree. Both six-call order arms, seven view/window witnesses per arm and strict native full-RGB assertions remain. The separate maximum-level record schema is diagnostic only and cannot accept pixel failures or attribute historical overlay pixels. Ordinary scenes keep their ScreenSaver selection and their own records; native F/Rust capture logic is unchanged.\n")
file(APPEND "${CMAKE_CURRENT_BINARY_DIR}/source-context-scope.txt"
    "A separate native-sample module aliases selectors ONLY in its original-class object, forwards exact native values/channel, and carries independent per-call shared state through content and sample completions. It records actual first-display/config identity and checked CPU BGRA rows under two extra read locks before/after original completion; every original BGR pixel must match the native input. Caller return/elapsed are saved before a separate two-second post-forward observation wait. Extra getters/copies/locks alter diagnostic timing; native RGB assertions, ordinary source/scene/Rust calls and old failures remain. No native lease/caller-stack pointer escapes, capture retry, pixel mask, compositor producer proof, callback drainage or P1 acceptance. A separate C++ CPU-only control checks late/reversed cross-thread association and owner release across two translation units. Actual native compilation/execution still requires the batch's real checks.\n")
