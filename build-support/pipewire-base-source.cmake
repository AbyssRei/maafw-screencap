# Test-only extraction of the fixed screenshot base; input/ConfigBase are not
# dependencies of this native screenshot context. No function body is rewritten.
set(_mfs_base "${MFS_UPSTREAM_ROOT}/source/MaaLinuxControlUnit/Base/UnitBase.h")
if(NOT CMAKE_SCRIPT_MODE_FILE)
    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${_mfs_base}")
endif()
file(READ "${_mfs_base}" _mfs_base_text)
string(REPLACE "\r\n" "\n" _mfs_base_text "${_mfs_base_text}")
string(FIND "${_mfs_base_text}" "class ScreencapBase\n" _mfs_begin)
string(FIND "${_mfs_base_text}" "class InputBase\n" _mfs_end)
if(_mfs_begin LESS 0 OR _mfs_end LESS_EQUAL _mfs_begin)
    message(FATAL_ERROR "Missing fixed screenshot base declaration")
endif()
math(EXPR _mfs_length "${_mfs_end}-${_mfs_begin}")
string(SUBSTRING "${_mfs_base_text}" ${_mfs_begin} ${_mfs_length} _mfs_base_body)
file(MAKE_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/overlay/Base")
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/overlay/Base/UnitBase.h"
    "#pragma once\n#include <opencv2/core.hpp>\n#include \"Common/Conf.h\"\nMAA_CTRL_UNIT_NS_BEGIN\n${_mfs_base_body}\nMAA_CTRL_UNIT_NS_END\n")
