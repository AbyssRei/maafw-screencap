cmake_minimum_required(VERSION 4.2)
project(mfs_custom_capture LANGUAGES CXX)
if(WIN32 AND (NOT CMAKE_GENERATOR STREQUAL "Visual Studio 18 2026" OR NOT CMAKE_SIZEOF_VOID_P EQUAL 8))
    message(FATAL_ERROR "Initial Windows custom acceptance requires x64 VS2026/MSBuild")
endif()
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_SCAN_FOR_MODULES OFF)
set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreadedDLL)
set(OpenCV_STATIC ON)
find_package(OpenCV REQUIRED COMPONENTS core imgproc imgcodecs)
foreach(_input MFS_UPSTREAM_ROOT MFS_MAAUTILS_ROOT MFS_MEOJSON_INCLUDE)
    if(NOT IS_DIRECTORY "${${_input}}")
        message(FATAL_ERROR "Explicit ${_input} is required")
    endif()
endforeach()
include("${CMAKE_CURRENT_SOURCE_DIR}/../../../build-support/custom-capture-source.cmake")
foreach(_native_source
    source/MaaCustomControlUnit/Manager/CustomControlUnitMgr.cpp
    source/MaaCustomControlUnit/Manager/CustomControlUnitMgr.h
    source/MaaCustomControlUnit/API/CustomControlUnitAPI.cpp
    source/include/Common/Conf.h
    include/MaaControlUnit/CustomControlUnitAPI.h
    include/MaaControlUnit/ControlUnitAPI.h
    include/MaaControlUnit/MaaControlUnitDef.h
    include/MaaFramework/Instance/MaaCustomController.h
    include/MaaFramework/MaaDef.h
    include/MaaFramework/MaaPort.h)
    configure_file("${MFS_UPSTREAM_ROOT}/${_native_source}"
        "${CMAKE_CURRENT_BINARY_DIR}/native-api-source/${_native_source}" COPYONLY)
endforeach()
add_library(mfs_custom_capture SHARED bridge.cpp
    "${MFS_UPSTREAM_ROOT}/source/MaaCustomControlUnit/Manager/CustomControlUnitMgr.cpp"
    "${MFS_UPSTREAM_ROOT}/source/MaaCustomControlUnit/API/CustomControlUnitAPI.cpp")
target_include_directories(mfs_custom_capture PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/overlay" "${MFS_UPSTREAM_ROOT}/source/MaaCustomControlUnit"
    "${MFS_UPSTREAM_ROOT}/source/include"
    "${CMAKE_CURRENT_BINARY_DIR}" "${MFS_UPSTREAM_ROOT}/include"
    "${MFS_MAAUTILS_ROOT}/include" "${MFS_MEOJSON_INCLUDE}" ${OpenCV_INCLUDE_DIRS})
target_compile_definitions(mfs_custom_capture PRIVATE MAA_FRAMEWORK_EXPORTS MAA_CONTROL_UNIT_EXPORTS
    MAA_VERSION="mfs-p1-custom-api" NOMINMAX WIN32_LEAN_AND_MEAN)
target_link_libraries(mfs_custom_capture PRIVATE ${OpenCV_LIBS})
if(MSVC)
    target_compile_options(mfs_custom_capture PRIVATE /utf-8 /EHsc)
endif()
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/source-scope.txt"
    "Exact fixed constructor/connect/connected/screencap/inactive/request_uuid/get_features/get_info/get_info_from_controller and raw C setter. Also compile complete unchanged original Custom API/manager and actual public base/mixin declarations, with logging-only overlay, for capture/identity/info virtual consumers and original C create/destroy. Real MaaUtils buffers, meojson and OpenCV. Rust unit uses uninvoked false input/app/shell slots solely to instantiate the actual interface. No input actions, framework loader or SDK provider acceptance. Mat and UUID actual caller assignment is inside the original native buffer lifetime, not deferred to virtual return; native probes reject outside-scope assignment. Opaque native heads add bookkeeping allocation, and independent Rust UUID storage adds a byte copy. No second pixel copy. JSON parse/object copy stays inside its temporary buffer, type replacement after it. Controlled native value/unit ledgers are not a general heap-leak proof or full P1 acceptance.\n")
