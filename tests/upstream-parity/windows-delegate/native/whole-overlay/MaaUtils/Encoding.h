// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <string>
#include <string_view>
#include "MaaUtils/Conf.h"
#include "whole-exclusions.hpp"
// Only start_app/stop_app reference this excluded conversion. Their original
// class bodies compile, but an accidental invocation fails this process case.
MAA_NS_BEGIN
inline std::wstring to_u16(std::string_view) {MfsWhole::application();}
MAA_NS_END
