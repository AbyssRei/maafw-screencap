// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "MaaControlUnit/Win32ControlUnitAPI.h"
#include <string>
namespace MfsDelegateFixture {
struct Plan { uint32_t flags=0,outcome=0,inactive=0; };
extern Plan plan;
extern bool module_loaded;
void event(const std::string&);
MaaWin32ControlUnitHandle create(void*,MaaWin32ScreencapMethod,MaaWin32InputMethod,MaaWin32InputMethod);
void destroy(MaaWin32ControlUnitHandle);
}
