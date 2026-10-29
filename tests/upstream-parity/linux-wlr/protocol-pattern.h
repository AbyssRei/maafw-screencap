// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Owned protocol fixture input, not a compositor or product format contract.
// The caller includes its native Wayland client/server protocol header first.
#pragma once
#include <stddef.h>
#include <stdint.h>
#include <string.h>

struct mfs_protocol_case {
    uint32_t format, channels, stride, inverted, index;
};
static inline struct mfs_protocol_case mfs_protocol_case(uint32_t index)
{
    const uint32_t formats[] = {WL_SHM_FORMAT_XRGB8888, WL_SHM_FORMAT_ARGB8888,
        WL_SHM_FORMAT_XBGR8888, WL_SHM_FORMAT_ABGR8888, WL_SHM_FORMAT_RGB888, WL_SHM_FORMAT_BGR888};
    const uint32_t kind = index / 4;
    const uint32_t channels = kind < 4 ? 4 : 3;
    const struct mfs_protocol_case result = {formats[kind], channels,
        7 * channels + ((index / 2) % 2 ? 13 : 0), index % 2, index};
    return result;
}
static inline uint8_t mfs_protocol_color(uint32_t x, uint32_t y, uint32_t component, uint32_t kind)
{
    return (uint8_t)((x * 17 + y * 31 + component * 47 + kind * 7) % 239 + 1);
}
static inline void mfs_protocol_fill(struct mfs_protocol_case c, uint8_t* bytes)
{
    memset(bytes, 0xa5, 5 * c.stride);
    const uint32_t kind = c.index / 4;
    const int bgr = kind < 2 || kind == 4;
    for (uint32_t row = 0; row < 5; ++row) {
        const uint32_t y = c.inverted ? 4 - row : row;
        for (uint32_t x = 0; x < 7; ++x) {
            uint8_t* pixel = bytes + row * c.stride + x * c.channels;
            pixel[0] = mfs_protocol_color(x,y,bgr ? 2 : 0,kind);
            pixel[1] = mfs_protocol_color(x,y,1,kind);
            pixel[2] = mfs_protocol_color(x,y,bgr ? 0 : 2,kind);
            if (c.channels == 4) { pixel[3] = (uint8_t)((x + 3 * y + kind) % 251 + 1); }
        }
    }
}
