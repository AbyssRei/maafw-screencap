// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Shared fixture provenance rules, not a capture/SDK geometry contract.
pub fn provenance(
    scene: &[u8],
    source: Option<&[u8]>,
    original_failed: bool,
    host_premise: bool,
) -> Result<String, &'static str> {
    match source {
        Some(value) if value != scene => return Err("source and scene geometry differ"),
        None if !original_failed => return Err("successful original oracle has no geometry"),
        _ => {}
    }
    let kind = match (source.is_some(), host_premise) {
        (false, _) => "missing",
        (true, true) => "host_expectation",
        (true, false) => "native_consumer_query",
    };
    Ok(format!(
        "geometry_origin=scene_host_expectation\nsource_geometry_present={}\nsource_geometry_kind={kind}\noriginal_oracle_failed={}\ngeometry_file_is_native_rust_observation=0\n",
        u8::from(source.is_some()),
        u8::from(original_failed)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn permits_independent_observation_only_after_explicit_original_failure() {
        assert!(provenance(b"scene", None, false, false).is_err());
        assert!(provenance(b"scene", None, false, true).is_err());
        assert_eq!(
            provenance(b"scene", None, true, false).unwrap(),
            "geometry_origin=scene_host_expectation\nsource_geometry_present=0\nsource_geometry_kind=missing\noriginal_oracle_failed=1\ngeometry_file_is_native_rust_observation=0\n"
        );
        for failed in [false, true] {
            assert!(provenance(b"scene", Some(b"different"), failed, false).is_err());
            assert!(provenance(b"scene", Some(b"different"), failed, true).is_err());
            let queried = provenance(b"scene", Some(b"scene"), failed, false).unwrap();
            assert!(queried.contains("source_geometry_kind=native_consumer_query\n"));
            assert!(queried.ends_with("geometry_file_is_native_rust_observation=0\n"));
            let expected = provenance(b"scene", Some(b"scene"), failed, true).unwrap();
            assert!(expected.contains("source_geometry_kind=host_expectation\n"));
        }
    }
}
