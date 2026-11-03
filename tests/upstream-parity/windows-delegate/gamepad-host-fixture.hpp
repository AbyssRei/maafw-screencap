// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "fixture.hpp"
#include <cstdint>
namespace MfsGamepadFixture {
// 0=true, 1=false, 2=connect exception, 3=create exception,
// 4=connect true but subsequent live connected observation is false.
extern unsigned mode,next_input;
void input_live(unsigned);
void input_drop(unsigned);
}
