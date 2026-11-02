// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "native-sample-state-control.hpp"
uint64_t MfsSampleControl::other_translation_unit_id() {
    const auto state=Scope::current();return state ? state->id : 0;
}
