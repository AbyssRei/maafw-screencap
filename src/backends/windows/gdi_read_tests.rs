// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;

#[test]
fn rejects_dimensions_and_lengths_that_cannot_reach_getbitmapbits() {
    for (width, height) in [(0, 1), (1, 0), (-1, 2), (i32::MAX, 1), (32768, 32768)] {
        assert!(checked_layout::<()>(width, height).is_err());
    }
    assert_eq!(checked_layout::<()>(37, 23).unwrap(), (148, 3404, 3404));
}

#[test]
fn rejects_short_reads_and_incompatible_device_bitmap_layouts() {
    assert!(matches!(
        check_read::<()>(7, 8),
        Err(CaptureError::IncompleteRead {
            copied: 7,
            required: 8
        })
    ));
    assert!(check_read::<()>(-1, 8).is_err());
    let mut bitmap = BITMAP {
        bmWidth: 37,
        bmHeight: 23,
        bmWidthBytes: 148,
        bmPlanes: 1,
        bmBitsPixel: 32,
        ..Default::default()
    };
    assert!(validate_bitmap::<()>(&bitmap, 37, 23, 148).is_ok());
    bitmap.bmBitsPixel = 24;
    assert!(matches!(
        validate_bitmap::<()>(&bitmap, 37, 23, 148),
        Err(CaptureError::UnsupportedBitmap)
    ));
    bitmap.bmBitsPixel = 32;
    bitmap.bmWidthBytes = 152;
    assert!(validate_bitmap::<()>(&bitmap, 37, 23, 148).is_err());
}
