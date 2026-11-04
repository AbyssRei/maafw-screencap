// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Actual original class RTTI and zero invocation of excluded input/application
// boundaries. The extra diagnostic DLL lease ends before unit destruction.
use super::super::whole_probe::Probe;
use super::*;
pub(super) struct Inspector {
    probe: Probe,
    rows: fs::File,
    selection: fs::File,
}
impl Inspector {
    pub(super) fn new(
        provider: &Path,
        directory: &Path,
        unit: NonNull<c_void>,
        method: u64,
    ) -> Result<Self> {
        let mut result = Self {
            probe: Probe::load(provider)?,
            rows: fs::File::create_new(directory.join("whole-source-types.tsv"))?,
            selection: fs::File::create_new(directory.join("whole-source-selected.tsv"))?,
        };
        result.record(0, unit)?;
        assert_eq!(result.selected(0, unit)?, 0);
        fs::write(
            directory.join("whole-source-selection.txt"),
            format!(
                "configured_mask={method}\nselection_observer=1\noriginal_class_api=1\ninput_none=1\n"
            ),
        )?;
        Ok(result)
    }
    fn record(&mut self, stage: u32, unit: NonNull<c_void>) -> Result<()> {
        let values = self.probe.inspect(unit);
        writeln!(
            self.rows,
            "{stage}\t{}",
            values.map(|v| v.to_string()).join("\t")
        )?;
        self.rows.flush()?;
        Ok(())
    }
    pub(super) fn finish(&mut self, unit: NonNull<c_void>) -> Result<()> {
        self.record(1, unit)
    }
    pub(super) fn selected(&mut self, stage: u32, unit: NonNull<c_void>) -> Result<u64> {
        let values = self.probe.selected(unit);
        writeln!(
            self.selection,
            "{stage}\t{}",
            values.map(|v| v.to_string()).join("\t")
        )?;
        self.selection.flush()?;
        Ok(values[2])
    }
}
