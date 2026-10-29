// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independent byte oracle for actual native protocol records, shared by tools
//! and the Rust native caller. Native producer/C++ oracle use their C headers.
pub const WIDTH: usize = 7;
pub const HEIGHT: usize = 5;
pub fn format(index: usize) -> u32 {
    [
        1,
        0,
        u32::from_le_bytes(*b"XB24"),
        u32::from_le_bytes(*b"AB24"),
        u32::from_le_bytes(*b"RG24"),
        u32::from_le_bytes(*b"BG24"),
    ][index / 4]
}
pub fn channels(index: usize) -> usize {
    if index / 4 < 4 { 4 } else { 3 }
}
pub fn stride(index: usize) -> usize {
    WIDTH * channels(index) + if (index / 2) % 2 == 1 { 13 } else { 0 }
}
fn color(x: usize, y: usize, channel: usize, kind: usize) -> u8 {
    u8::try_from((x * 17 + y * 31 + channel * 47 + kind * 7) % 239 + 1).unwrap()
}
pub fn native(index: usize) -> Vec<u8> {
    let kind = index / 4;
    let mut bytes = vec![0xa5; HEIGHT * stride(index)];
    let bgr = kind < 2 || kind == 4;
    for row in 0..HEIGHT {
        for x in 0..WIDTH {
            let y = if index % 2 == 1 {
                HEIGHT - 1 - row
            } else {
                row
            };
            let base = row * stride(index) + x * channels(index);
            bytes[base] = color(x, y, if bgr { 2 } else { 0 }, kind);
            bytes[base + 1] = color(x, y, 1, kind);
            bytes[base + 2] = color(x, y, if bgr { 0 } else { 2 }, kind);
            if channels(index) == 4 {
                bytes[base + 3] = u8::try_from((x + 3 * y + kind) % 251 + 1).unwrap();
            }
        }
    }
    bytes
}
pub fn raw(index: usize) -> Vec<u8> {
    native(index)[..WIDTH * HEIGHT * channels(index)].to_vec()
}
pub fn legacy(index: usize) -> Vec<u8> {
    let raw = raw(index);
    let swap = matches!(index / 4, 2 | 3 | 5);
    let mut result = Vec::new();
    for pixel in raw.chunks_exact(channels(index)) {
        for channel in 0..3 {
            result.push(pixel[if swap { 2 - channel } else { channel }]);
        }
    }
    result
}
pub fn target(index: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            for channel in 0..3 {
                bytes.push(color(x, y, 2 - channel, index / 4));
            }
        }
    }
    bytes
}
