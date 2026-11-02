// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Native parser branch regression. No stream/callback/capture is performed.
use super::*;
use crate::backends::linux::pipewire::Format;
use std::fmt::Write;

#[test]
#[ignore = "requires exact original native format branch fixtures"]
fn matches_original_standard_result_and_fallback_selection() {
    let directory = std::path::PathBuf::from(
        env::var_os("MFS_P1_PIPEWIRE_FORMAT_BRANCHES")
            .expect("explicit native parser branch matrix"),
    );
    let source = directory.join("source");
    let ids = fs::read_to_string(source.join("ids.tsv")).unwrap();
    assert_eq!(
        ids,
        format!(
            "bgra\tnv12\tunknown\n{}\t{}\t{}\n",
            spa::sys::SPA_VIDEO_FORMAT_BGRA,
            spa::sys::SPA_VIDEO_FORMAT_NV12,
            spa::sys::SPA_VIDEO_FORMAT_UNKNOWN
        )
    );
    let records = fs::read_to_string(source.join("cases.tsv")).unwrap();
    let mut lines = records.lines();
    assert_eq!(
        lines.next().unwrap(),
        "index\tfirst\thas_second\tsecond\tsecond_choice\tnative_result\tnative_width\tnative_height\tnative_format\tsource_width\tsource_height\tsource_format\tsource_accepted"
    );
    let mut output = String::from("index\trust_accepted\trust_width\trust_height\trust_format\n");
    let (mut cases, mut differences) = (0, 0);
    for line in lines {
        let values: Vec<i64> = line.split('\t').map(|v| v.parse().unwrap()).collect();
        assert_eq!(values.len(), 13);
        assert_eq!(values[0], cases);
        assert!(matches!(values[12], 0 | 1));
        let encoded = fs::read(source.join(format!("case-{cases}.pod"))).unwrap();
        assert!((8..=4096).contains(&encoded.len()));
        assert_eq!(
            usize::try_from(u32::from_ne_bytes(encoded[..4].try_into().unwrap()))
                .unwrap()
                .checked_add(8)
                .unwrap(),
            encoded.len()
        );
        let mut storage = [0_u64; 512];
        // SAFETY: Complete trusted original native-builder fixture, checked
        // length, aligned initialized owner, and no pointer outlives this call.
        let parsed = unsafe {
            let bytes = std::slice::from_raw_parts_mut(storage.as_mut_ptr().cast::<u8>(), 4096);
            bytes[..encoded.len()].copy_from_slice(&encoded);
            pod::parse(storage.as_ptr().cast())
        };
        let expected = (values[12] == 1).then(|| {
            (
                u32::try_from(values[9]).unwrap(),
                u32::try_from(values[10]).unwrap(),
                pod::to_format(u32::try_from(values[11]).unwrap()),
            )
        });
        let (accepted, width, height, format) = match parsed {
            None => (0, 0, 0, spa::sys::SPA_VIDEO_FORMAT_UNKNOWN),
            Some((width, height, format)) => {
                let numeric = match format {
                    Format::Bgra => spa::sys::SPA_VIDEO_FORMAT_BGRA,
                    Format::Bgrx => spa::sys::SPA_VIDEO_FORMAT_BGRx,
                    Format::Rgba => spa::sys::SPA_VIDEO_FORMAT_RGBA,
                    Format::Rgbx => spa::sys::SPA_VIDEO_FORMAT_RGBx,
                    Format::Rgb => spa::sys::SPA_VIDEO_FORMAT_RGB,
                    Format::Bgr => spa::sys::SPA_VIDEO_FORMAT_BGR,
                    Format::Yuy2 => spa::sys::SPA_VIDEO_FORMAT_YUY2,
                    Format::Unknown => spa::sys::SPA_VIDEO_FORMAT_UNKNOWN,
                };
                (1, width, height, numeric)
            }
        };
        writeln!(
            &mut output,
            "{cases}\t{accepted}\t{width}\t{height}\t{format}"
        )
        .unwrap();
        eprintln!("native parser branch case={cases} source={expected:?} rust={parsed:?}");
        differences += i64::from(parsed != expected);
        cases += 1;
    }
    fs::write(directory.join("rust.tsv"), output).unwrap();
    eprintln!(
        "native PipeWire parser branches cases={cases} differences={differences} capture=0 recorded"
    );
    assert_eq!(cases, 8);
    assert_eq!(
        differences, 0,
        "standard-result/fallback selection differs from original"
    );
}
