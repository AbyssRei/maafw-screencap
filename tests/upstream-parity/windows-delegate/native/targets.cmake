# Explicit acceptance-only native module boundaries; never part of product linkage.
file(GENERATE OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/native-toolchain-$<CONFIG>.txt" CONTENT
    "compiler=${CMAKE_CXX_COMPILER}\ncompiler_id=${CMAKE_CXX_COMPILER_ID}\ncompiler_version=${CMAKE_CXX_COMPILER_VERSION}\nsdk=${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION}\nconfiguration=$<CONFIG>\nmsvc_runtime=$<TARGET_PROPERTY:mfs_delegate_bridge,MSVC_RUNTIME_LIBRARY>\nbridge_options=$<TARGET_PROPERTY:mfs_delegate_bridge,COMPILE_OPTIONS>\nsource_provider_options=$<TARGET_PROPERTY:mfs_delegate_source,COMPILE_OPTIONS>\nrust_provider_options=$<TARGET_PROPERTY:mfs_delegate_rust,COMPILE_OPTIONS>\n")
include("${CMAKE_CURRENT_SOURCE_DIR}/../../../build-support/manager-source.cmake")
find_package(OpenCV REQUIRED COMPONENTS core imgproc)
add_library(mfs_delegate_bridge SHARED native/bridge.cpp)
target_compile_definitions(mfs_delegate_bridge PRIVATE MFS_BRIDGE_EXPORTS)
foreach(_provider source missing-create missing-destroy rust)
    string(REPLACE "-" "_" _target "mfs_delegate_${_provider}")
    add_library(${_target} SHARED native/provider.cpp)
    if(_provider STREQUAL "rust")
        target_compile_definitions(${_target} PRIVATE MFS_RUST_PROVIDER)
    else()
        foreach(_backend GdiScreencap PrintWindowScreencap ScreenDCScreencap FramePoolScreencap DesktopDupScreencap DesktopDupWindowScreencap PseudoMinimizeHelper)
            target_sources(${_target} PRIVATE "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit/Screencap/${_backend}.cpp")
        endforeach()
        target_compile_definitions(${_target} PRIVATE MAA_FRAMEPOOL_SCREENCAP_AVAILABLE=1)
        target_link_libraries(${_target} PRIVATE Dwmapi D3D11 Dxgi WindowsApp RuntimeObject CoreMessaging)
    endif()
    target_compile_definitions(${_target} PRIVATE MAA_CONTROL_UNIT_EXPORTS)
    if(_provider STREQUAL "missing-create")
        target_compile_definitions(${_target} PRIVATE MFS_NO_CREATE)
        set(_exports "MaaWin32ControlUnitDestroy")
    elseif(_provider STREQUAL "missing-destroy")
        target_compile_definitions(${_target} PRIVATE MFS_NO_DESTROY)
        set(_exports "MaaWin32ControlUnitCreate")
    else()
        set(_exports "MaaWin32ControlUnitCreate\nMaaWin32ControlUnitDestroy")
    endif()
    file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/${_target}.def" "EXPORTS\n${_exports}\n")
    target_sources(${_target} PRIVATE "${CMAKE_CURRENT_BINARY_DIR}/${_target}.def")
    target_link_libraries(${_target} PRIVATE mfs_delegate_bridge Gdi32 User32)
endforeach()
foreach(_target mfs_delegate_bridge mfs_delegate_source mfs_delegate_missing_create mfs_delegate_missing_destroy mfs_delegate_rust)
    target_include_directories(${_target} PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/native"
        "${CMAKE_CURRENT_SOURCE_DIR}/native/overlay" "${CMAKE_CURRENT_BINARY_DIR}"
        "${MFS_UPSTREAM_ROOT}/include" "${MFS_UPSTREAM_ROOT}/source/include"
        "${MFS_UPSTREAM_ROOT}/source/MaaWin32ControlUnit" "${MFS_MAAUTILS_ROOT}/include"
        "${MFS_MEOJSON_INCLUDE}" ${OpenCV_INCLUDE_DIRS})
    target_compile_definitions(${_target} PRIVATE NOMINMAX WIN32_LEAN_AND_MEAN BOOST_ALL_NO_LIB)
    # Original extern-C declarations can call throwing C++ constructors. Do
    # not enable /EHc's assumption that every extern-C function is nothrow.
    target_compile_options(${_target} PRIVATE /utf-8 /EHs)
    target_link_libraries(${_target} PRIVATE ${OpenCV_LIBS})
endforeach()
# This overlay contains ONLY a logger. Actual Boost DLL/LibraryHolder are used.
add_executable(mfs_delegate_native_source native/source.cpp)
target_include_directories(mfs_delegate_native_source PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/native"
    "${CMAKE_CURRENT_SOURCE_DIR}/native/overlay" "${CMAKE_CURRENT_BINARY_DIR}"
    "${MFS_UPSTREAM_ROOT}/include" "${MFS_MAAUTILS_ROOT}/include"
    "${MFS_MEOJSON_INCLUDE}" ${OpenCV_INCLUDE_DIRS})
target_compile_definitions(mfs_delegate_native_source PRIVATE NOMINMAX WIN32_LEAN_AND_MEAN BOOST_ALL_NO_LIB)
target_compile_options(mfs_delegate_native_source PRIVATE /utf-8 /EHs)
target_link_libraries(mfs_delegate_native_source PRIVATE mfs_delegate_bridge Boost::dll Boost::function ${OpenCV_LIBS})
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/native-scope.txt"
    "Actual flags=0 Windows DLL loads, original LibraryHolder/Boost DLL and loader/outer capture bodies. Real original interface ABI. Source provider wraps extracted manager bodies and now compiles all six unedited original screenshot methods/helper; input=None shim and logger/path injection remain. Existing actual capture matrix exercises only GDI; compiling other backends does not prove native creation or capture. Rust provider is an acceptance-only C++ ABI facade calling the actual private Rust manager/factory via synchronous borrowed host callbacks, forwarding null HWND and all selector masks without constructor validation. It copies Rust output to provider Mat; bridge then copies provider pixels once into independent CPU bytes. These two boundary copies are extra work, not eliminated work. Full six-method runtime/framework integration and P1 remain pending.\n")
file(APPEND "${CMAKE_CURRENT_BINARY_DIR}/native-scope.txt"
    "MaaWin32ControlUnitCreate/Destroy compile the complete fixed original source bodies without rewriting statements. The concrete Win32ControlUnitMgr name denotes the explicitly scoped capture-only source/Rust manager facade with original Win32ControlUnitAPI inheritance; constructor instrumentation and input=None boundary remain explicit. Real std::make_unique/release, pointer forwarding, null-safe virtual delete and exception unwinding are consumed, not handwritten factory algorithm substitutes.\n")
