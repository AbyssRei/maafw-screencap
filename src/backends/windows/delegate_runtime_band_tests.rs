// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Independent fixture queries; never drain the UI queue around capture calls.
use crate::backends::windows::parity_adapter::screen_fixture::Fixture;
use crate::platform::windows::window_api::{Point, SystemWindow, WindowApi};
use std::{error::Error, ffi::c_void, fs, io::Write, path::Path};

pub(super) struct Band {
    rows: fs::File,
    root: std::path::PathBuf,
    expected: [i64; 10],
    count: u64,
    valid: bool,
}

pub(super) fn snapshot(window: *mut c_void) -> Result<[i64; 10], Box<dyn Error>> {
    let outer = SystemWindow
        .window_rect(window)
        .map_err(|e| format!("band outer {e}"))?;
    let client = SystemWindow
        .client_rect(window)
        .map_err(|e| format!("band client {e}"))?;
    let origin = SystemWindow
        .origin(window, Point::default())
        .map_err(|e| format!("band origin {e}"))?;
    Ok([
        outer.left,
        outer.top,
        outer.right,
        outer.bottom,
        client.left,
        client.top,
        client.right,
        client.bottom,
        origin.x,
        origin.y,
    ]
    .map(i64::from))
}

impl Band {
    pub(super) fn prepare(fixture: &Fixture, root: &Path) -> Result<Self, Box<dyn Error>> {
        let before = fixture.geometry([0; 6])?;
        let x = before[4].checked_add(64).ok_or("band x overflow")?;
        let y = before[5].checked_add(64).ok_or("band y overflow")?;
        if x < i64::from(i32::MIN) + 6
            || y < i64::from(i32::MIN) + 6
            || x > i64::from(i32::MAX) - 102
            || y > i64::from(i32::MAX) - 70
        {
            return Err("band native coordinate range".into());
        }
        fixture.geometry([4, 0x9400_0000, x, y, 96, 64])?;
        let after = fixture.geometry([2, 0, 0, 0, 0, 0])?;
        let expected = [x - 6, y - 6, x + 102, y + 70, 0, 0, 96, 64, x, y];
        let observed: [i64; 10] = after[8..18].try_into()?;
        if before[..8] != after[..8]
            || observed != expected
            || after[18] != 0x9400_0000
            || after[19] & 8 != 8
            || after[27..29] != [1, 0]
        {
            return Err("manager band full-client/native setup differs".into());
        }
        let mut setup = fs::File::create_new(root.join("client-band-setup.tsv"))?;
        for row in [before, after] {
            writeln!(setup, "{}", row.map(|v| v.to_string()).join("\t"))?;
        }
        Ok(Self {
            rows: fs::File::create_new(root.join("client-band-boundaries.tsv"))?,
            root: root.to_owned(),
            expected,
            count: 0,
            valid: true,
        })
    }

    pub(super) fn record(
        &mut self,
        window: *mut c_void,
        cycle: u64,
        operation: u64,
        before: [i64; 10],
    ) -> Result<(), Box<dyn Error>> {
        let after = snapshot(window)?;
        self.valid &= before == self.expected && after == self.expected;
        writeln!(
            self.rows,
            "{cycle}\t{operation}\t{}\t{}",
            before.map(|v| v.to_string()).join("\t"),
            after.map(|v| v.to_string()).join("\t")
        )?;
        self.rows.flush()?;
        self.count = self.count.checked_add(1).ok_or("band row overflow")?;
        Ok(())
    }

    pub(super) fn finish(self) -> Result<bool, Box<dyn Error>> {
        let valid = self.valid && self.count == 120;
        fs::write(
            self.root.join("client-band-summary.txt"),
            format!(
                "schema=1\nrows={}\ngeometry_valid={}\nnonclient_band=6\ntarget_inset=0\n",
                self.count,
                u8::from(valid)
            ),
        )?;
        Ok(valid)
    }
}
