// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Actual original class RTTI and zero invocation of excluded input/application
// boundaries. The extra diagnostic DLL lease ends before unit destruction.
use super::super::whole_probe::Probe;
use super::*;
pub(super) struct Inspector {
    probe: Probe,
    rows: fs::File,
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
        };
        result.record(0, unit)?;
        fs::write(
            directory.join("whole-source-selection.txt"),
            format!(
                "single_method_mask={method}\nselection_observer=0\noriginal_class_api=1\ninput_none=1\n"
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
}
