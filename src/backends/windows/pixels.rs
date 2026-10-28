// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
/// P1 private conversion boundary, called at each original conversion point.
/// The result must own storage independent of this immutable input borrow.
pub(crate) trait BgraConverter {
    type Image;
    type Error;
    fn bgra_to_bgr(&mut self, input: BgraView<'_>) -> Result<Self::Image, Self::Error>;
}

pub(crate) struct BgraView<'a> {
    pub pixels: &'a [u8],
    pub width: i32,
    pub height: i32,
    pub stride: usize,
}
