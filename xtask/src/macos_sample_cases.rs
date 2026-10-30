// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Test input specification only, shared by the owning native test and checker.
pub(crate) const CASES: [(usize, usize, usize); 4] =
    [(7, 5, 64), (47, 31, 64), (65, 3, 128), (1, 1, 256)];
pub(crate) const CYCLES: usize = 20;
pub(crate) fn raw(case: usize, cycle: usize) -> Vec<u8> {
    let (width, height, _) = CASES[case];
    let mut bytes = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            bytes.extend_from_slice(&[
                ((x * 17 + y * 3 + cycle * 11 + case * 5) % 256) as u8,
                ((x * 7 + y * 19 + cycle * 13 + case * 11) % 256) as u8,
                ((x * 23 + y * 5 + cycle * 3 + case * 17) % 256) as u8,
                [0, 127, 255][(x + y + cycle + case) % 3],
            ]);
        }
    }
    bytes
}
pub(crate) fn bgr(raw: &[u8]) -> Vec<u8> {
    raw.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|pixel| pixel[..3].iter().copied())
        .collect()
}
