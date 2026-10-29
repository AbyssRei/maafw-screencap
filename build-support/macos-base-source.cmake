# Reference-only extraction of the complete fixed screenshot base. Input and
# framework declarations stay outside this original capture context.
set(_mfs_base "${MFS_UPSTREAM_ROOT}/source/MaaMacOSControlUnit/Base/UnitBase.h")
if(NOT CMAKE_SCRIPT_MODE_FILE)
    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${_mfs_base}")
endif()
file(READ "${_mfs_base}" _mfs_text)
string(REPLACE "\r\n" "\n" _mfs_text "${_mfs_text}")
string(FIND "${_mfs_text}" "class ScreencapBase\n" _mfs_begin)
string(FIND "${_mfs_text}" "class InputBase\n" _mfs_end)
if(_mfs_begin LESS 0 OR _mfs_end LESS_EQUAL _mfs_begin)
    message(FATAL_ERROR "Missing fixed macOS screenshot base declaration")
endif()
math(EXPR _mfs_length "${_mfs_end}-${_mfs_begin}")
string(SUBSTRING "${_mfs_text}" ${_mfs_begin} ${_mfs_length} _mfs_body)
file(MAKE_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/overlay/Base")
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/overlay/Base/UnitBase.h"
    "#pragma once\n#include <optional>\n#include <opencv2/core.hpp>\n#include \"Common/Conf.h\"\nMAA_CTRL_UNIT_NS_BEGIN\n${_mfs_body}\nMAA_CTRL_UNIT_NS_END\n")
