// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#ifndef MFS_PIPEWIRE_FORMAT_PATTERN_H
#define MFS_PIPEWIRE_FORMAT_PATTERN_H
#include <stddef.h>
#include <stdint.h>
#include <string.h>
// Test-only raw bytes, not a color encoder. YUY2 contains explicit Y/U/Y/V
// samples; its expected BGR is independently recomputed by the records checker.
static inline void mfs_pw_format_pattern(uint8_t *pixels, size_t channels,
                                         size_t stride, unsigned phase)
{
    memset(pixels, 0xa5, stride * 24);
    for (size_t y = 0; y < 24; ++y)
        for (size_t x = 0; x < 28; ++x)
            for (size_t c = 0; c < channels; ++c)
                pixels[y * stride + x * channels + c] =
                    (uint8_t)((y * 29 + x * 17 + c * 53 + phase * 71) & 255);
}
#endif
