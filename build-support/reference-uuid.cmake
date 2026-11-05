# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Original dependency implementation, not a replacement UUID provider.
foreach(_input MFS_MAAUTILS_ROOT MFS_BOOST_ROOT)
    if(NOT IS_DIRECTORY "${${_input}}")
        message(FATAL_ERROR "Explicit ${_input} is required for original UUID acceptance")
    endif()
endforeach()
set(BOOST_INCLUDE_LIBRARIES uuid)
set(BOOST_ENABLE_CMAKE ON)
add_subdirectory("${MFS_BOOST_ROOT}" "${CMAKE_CURRENT_BINARY_DIR}/Boost" EXCLUDE_FROM_ALL)
add_library(mfs_reference_uuid STATIC "${MFS_MAAUTILS_ROOT}/source/Uuid/Uuid.cpp")
target_include_directories(mfs_reference_uuid PUBLIC "${MFS_MAAUTILS_ROOT}/include")
target_compile_definitions(mfs_reference_uuid PUBLIC MAA_UTILS_EXPORTS)
target_link_libraries(mfs_reference_uuid PRIVATE Boost::uuid)
