// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;

#[cfg(feature = "prototype-windows")]
#[path = "print_window_parity_tests.rs"]
mod parity;

#[test]
fn clips_negative_popup_origins_and_excludes_edge_touching_windows() {
    assert_eq!(
        intersection::<()>((37, 23), (25, 17), (-7, -5)).unwrap(),
        Some(Overlay {
            source_x: 7,
            source_y: 5,
            destination_x: 0,
            destination_y: 0,
            width: 18,
            height: 12
        })
    );
    assert_eq!(
        intersection::<()>((37, 23), (25, 17), (30, 20)).unwrap(),
        Some(Overlay {
            source_x: 0,
            source_y: 0,
            destination_x: 30,
            destination_y: 20,
            width: 7,
            height: 3
        })
    );
    for offset in [(37, 0), (-25, 0), (0, 23), (0, -17)] {
        assert_eq!(
            intersection::<()>((37, 23), (25, 17), offset).unwrap(),
            None
        );
    }
}

#[test]
fn preserves_defined_empty_intersections_and_rejects_undefined_origin_subtraction() {
    assert_eq!(
        intersection::<()>((37, 23), (25, 17), (i64::from(i32::MAX), 0)).unwrap(),
        None
    );
    assert_eq!(
        intersection::<()>((37, 23), (25, 17), (i64::from(i32::MIN), 0)).unwrap(),
        None
    );
    assert!(intersection::<()>((37, 23), (25, 17), (i64::from(i32::MIN) - 1, 0)).is_err());
    assert!(layout::<()>(i32::MAX, i32::MAX).is_err());
    assert!(layout::<()>(0, 17).is_err());
    assert_eq!(layout::<()>(37, 23).unwrap(), (148, 3404));
}
