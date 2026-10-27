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

add_library(mfs_p0_adapter SHARED interop.cpp)
target_link_libraries(mfs_p0_adapter PRIVATE ${OpenCV_LIBS})
target_include_directories(mfs_p0_adapter PRIVATE ${OpenCV_INCLUDE_DIRS})

if(MFS_BUILD_REFERENCE)
foreach(input MFS_UPSTREAM_ROOT MFS_MAAUTILS_ROOT MFS_BOOST_ROOT)
    if(NOT IS_DIRECTORY "${${input}}")
        message(FATAL_ERROR "Explicit external input ${input} is required")
    endif()
endforeach()
set(BOOST_INCLUDE_LIBRARIES regex)
set(BOOST_BUILD_TESTING OFF)
set(BOOST_SKIP_INSTALL_RULES ON)
add_subdirectory("${MFS_BOOST_ROOT}" "${CMAKE_BINARY_DIR}/boost")

# Build only the utilities consumed by screenshot logging. The upstream utility
# project also configures OCR, inference and unrelated process dependencies.
# These source files are compiled without edits; this is a build adaptation.
add_library(mfs_reference_utils SHARED
    "${MFS_MAAUTILS_ROOT}/source/Logger/Logger.cpp"
    "${MFS_MAAUTILS_ROOT}/source/Platform/PlatformWin32.cpp")
target_compile_definitions(mfs_reference_utils PRIVATE MAA_UTILS_EXPORTS)
target_include_directories(mfs_reference_utils PUBLIC "${MFS_MAAUTILS_ROOT}/include" ${OpenCV_INCLUDE_DIRS})
target_link_libraries(mfs_reference_utils PRIVATE ${OpenCV_LIBS} Boost::regex Psapi)
target_compile_definitions(mfs_reference_utils PRIVATE BOOST_ALL_NO_LIB)

add_executable(mfs_reference_u
    reference.cpp
    "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/GdiScreencap.cpp")
target_include_directories(mfs_reference_u PRIVATE
    "${MFS_UPSTREAM_ROOT}/include"
    "${MFS_UPSTREAM_ROOT}/source/include"
    "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit")
target_link_libraries(mfs_reference_u PRIVATE mfs_reference_utils ${OpenCV_LIBS} Gdi32 User32)
endif()

file(GENERATE OUTPUT "${CMAKE_BINARY_DIR}/toolchain-$<CONFIG>.txt" CONTENT
    "generator=${CMAKE_GENERATOR}\ninstance=${CMAKE_GENERATOR_INSTANCE}\nplatform=${CMAKE_VS_PLATFORM_NAME}\ncompiler=${CMAKE_CXX_COMPILER}\ncompiler_version=${CMAKE_CXX_COMPILER_VERSION}\nsdk=${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION}\nruntime=${CMAKE_MSVC_RUNTIME_LIBRARY}\n")
