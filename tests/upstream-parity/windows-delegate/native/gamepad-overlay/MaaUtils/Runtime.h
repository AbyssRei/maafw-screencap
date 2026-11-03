// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <filesystem>
// Identified directory only. Actual LibraryHolder/Boost loads are unchanged.
namespace MaaNS {
inline std::filesystem::path directory;
inline std::filesystem::path library_dir() { return directory; }
}
