// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Same explicit logging suppression as the existing native source context.
#include "../../overlay/MaaUtils/Logger.h"
#define LogFunc MfsPipeWireLog::sink
#define VAR_VOIDP(x) x
