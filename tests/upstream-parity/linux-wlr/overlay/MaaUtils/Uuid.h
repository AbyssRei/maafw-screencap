// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "Common/Conf.h"
#include <string>
// Reference-only external name-provider boundary. The original MemfdBuffer
// body stays unchanged; this kernel-random provider replaces the unrelated
// Boost/MaaUtils implementation, not fd creation/mapping or capture behavior.
MAA_CTRL_UNIT_NS_BEGIN
std::string make_uuid();
MAA_CTRL_UNIT_NS_END
