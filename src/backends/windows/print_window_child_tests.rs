// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;

#[test]
#[ignore = "explicit original PrintWindow real-child/root-owned-popup parity"]
fn captures_the_child_and_composes_only_the_root_owned_popup() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    for (offset, popup, mode, conversions, overlays) in [
        ((0, 0), (25, 17), 0, 1, 0),
        ((5, 3), (25, 17), 1, 2, 1),
        ((-7, -5), (25, 17), 1, 2, 1),
        ((37, 0), (25, 17), 1, 2, 0),
        ((5, 3), (25, 17), 2, 1, 0),
        ((5, 3), (0, 17), 1, 1, 0),
    ] {
        let owners = adapter.image_owners()?;
        let all_resources = adapter.objects();
        let mut retained = None;
        for cycle in 0..20 {
            let mut fixture = adapter.print_child_fixture(popup, offset, mode)?;
            // SAFETY: Query a live fixture-owned child on its owning thread.
            let root = unsafe { SystemGdi.root_owner(fixture.hwnd()) };
            assert!(!root.is_null());
            assert_ne!(
                root,
                fixture.hwnd(),
                "this case must exercise a real child target"
            );
            if mode == 0 {
                // SAFETY: The same live fixture owns its root without a popup.
                assert_eq!(unsafe { SystemGdi.last_popup(root) }, root);
            }
            let baseline = adapter.objects();
            let original = adapter.print_reference(fixture.hwnd()).map_err(|status| {
                format!("original child: mode={mode}, offset={offset:?}, cycle={cycle}: {status}")
            })?;
            let mut converter = Observed {
                adapter: &adapter,
                baseline,
                conversions: 0,
                overlays: 0,
                fail_at: 0,
            };
            let actual = PrintWindowCapture::new(fixture.hwnd()).capture(&mut converter);
            assert!(actual.main_cleanup.0.iter().all(Option::is_none));
            assert!(actual.popup_cleanup.0.iter().all(Option::is_none));
            let image = actual
                .image
                .map_err(|error| format!("Rust child: {error:?}"))?;
            assert_eq!(converter.conversions, conversions);
            assert_eq!(converter.overlays, overlays);
            if mode == 1 && popup.0 == 0 {
                assert!(matches!(
                    actual.popup_failure,
                    Some(CaptureError::InvalidDimensions)
                ));
            } else {
                assert!(actual.popup_failure.is_none());
            }
            assert_eq!(adapter.objects(), baseline);
            assert_eq!(original.dimensions(), (37, 23));
            assert_pixels(&original, offset, popup, mode);
            assert_pixels(&image, offset, popup, mode);
            assert_eq!(image.bytes(), original.bytes());
            let rendered = fixture.stats()?;
            assert_eq!(rendered[4], 0);
            assert!(
                rendered[0] + rendered[1] > 0,
                "the child fixture must render its own pattern"
            );
            drop(original);
            fixture.close()?;
            assert_eq!(adapter.objects(), all_resources);
            assert_pixels(&image, offset, popup, mode);
            if retained.is_none() {
                retained = Some(image);
            } else {
                drop(image);
            }
            assert_eq!(adapter.image_owners()?, owners + 1);
        }
        assert_pixels(retained.as_ref().unwrap(), offset, popup, mode);
        drop(retained);
        assert_eq!(adapter.image_owners()?, owners);
        println!(
            "PrintWindow child parity: mode={mode}, offset={offset:?}, popup={popup:?}, 20 actual child/root-owner lifecycles"
        );
    }
    Ok(())
}
