// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual display/owned-window point geometry, shared only with acceptance tests.
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone, Debug)]
pub(crate) struct Geometry {
    pub width: usize,
    pub height: usize,
    pub window: u32,
    pub pid: u32,
    pub display: u32,
    pub region: [usize; 4],
}
pub(crate) fn parse(text: &str) -> Result<Geometry> {
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() != 1 {
        return Err("expected one native first-display geometry row".into());
    }
    let v = lines[0].split('\t').collect::<Vec<_>>();
    if v.len() != 16 {
        return Err("native first-display geometry column count differs".into());
    }
    let integer = |index: usize| -> Result<u32> {
        let value = v[index];
        if value.is_empty()
            || !value.bytes().all(|b| b.is_ascii_digit())
            || value.len() > 1 && value.starts_with('0')
        {
            return Err("invalid native display integer witness".into());
        }
        Ok(value.parse()?)
    };
    let count = integer(0)?;
    let display = integer(1)?;
    let width = usize::try_from(integer(2)?)?;
    let height = usize::try_from(integer(3)?)?;
    let window = integer(8)?;
    let pid = integer(9)?;
    if count == 0
        || display == 0
        || window == 0
        || pid == 0
        || width == 0
        || height == 0
        || width > i32::MAX as usize
        || height > i32::MAX as usize
        || integer(14)? != 1
        || integer(15)? != 2
    {
        return Err("invalid native display identity/configuration/visibility".into());
    }
    let mut point = [0_f64; 8];
    for (output, index) in point.iter_mut().zip([4, 5, 6, 7, 10, 11, 12, 13]) {
        *output = v[index].parse()?;
        if !output.is_finite() || output.abs() > 1_000_000_000.0 {
            return Err("nonfinite or unrepresentable native point geometry".into());
        }
    }
    let [dx, dy, dw, dh, wx, wy, ww, wh] = point;
    if dw <= 0.0 || dh <= 0.0 || ww != 47.0 || wh != 31.0 {
        return Err("owned display patch point extent differs".into());
    }
    // Use actual original configured dimensions. Do not reinterpret SCDisplay
    // width/height (documented points) as physical pixels or change U's config.
    let sx = width as f64 / dw;
    let sy = height as f64 / dh;
    let edges = [
        ((wx + 2.0 - dx) * sx).ceil(),
        ((wy + 2.0 - dy) * sy).ceil(),
        ((wx + ww - 2.0 - dx) * sx).floor(),
        ((wy + wh - 2.0 - dy) * sy).floor(),
    ];
    let mut region = [0; 4];
    for (output, edge) in region.iter_mut().zip(edges) {
        if !edge.is_finite() || edge < 0.0 || edge > i32::MAX as f64 {
            return Err("display patch edge overflow".into());
        }
        *output = edge as usize;
    }
    if region[0] >= region[2] || region[1] >= region[3] || region[2] > width || region[3] > height {
        return Err("owned patch lies outside original first-display output".into());
    }
    Ok(Geometry {
        width,
        height,
        window,
        pid,
        display,
        region,
    })
}
impl Geometry {
    pub fn span(&self, channels: usize) -> Result<usize> {
        self.width
            .checked_mul(self.height)
            .and_then(|v| v.checked_mul(channels))
            .ok_or_else(|| "native display pixel span overflow".into())
    }
    pub fn pixels(&self, bytes: &[u8], channels: usize) -> Result<()> {
        if !matches!(channels, 3 | 4) || bytes.len() != self.span(channels)? {
            return Err("native display full-image layout differs".into());
        }
        for y in self.region[1]..self.region[3] {
            for x in self.region[0]..self.region[2] {
                let offset = (y * self.width + x) * channels;
                if bytes[offset..offset + 3] != [0, 255, 0]
                    || channels == 4 && bytes[offset + 3] != 255
                {
                    return Err("native first-display owned green patch differs".into());
                }
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_offscreen_replaced_targets_nonfinite_extents_and_wrong_pixels() {
        let row = "1\t17\t640\t480\t-640\t0\t640\t480\t91\t7\t-500\t100\t47\t31\t1\t2\n";
        let g = parse(row).unwrap();
        assert_eq!(g.region, [142, 102, 185, 129]);
        assert_eq!((g.window, g.pid, g.display), (91, 7, 17));
        for bad in [
            row.replace("-500", "500"),
            row.replace("\t640\t480\t91", "\tNaN\t480\t91"),
            row.replace("\t1\t2\n", "\t0\t2\n"),
            row.replace("\t91\t7", "\t4294967296\t7"),
        ] {
            assert!(parse(&bad).is_err());
        }
        let mut image = vec![0; g.span(3).unwrap()];
        for y in g.region[1]..g.region[3] {
            for x in g.region[0]..g.region[2] {
                image[(y * g.width + x) * 3 + 1] = 255;
            }
        }
        g.pixels(&image, 3).unwrap();
        image[(102 * 640 + 142) * 3 + 1] = 0;
        assert!(g.pixels(&image, 3).is_err());
        assert!(g.pixels(&image[..image.len() - 1], 3).is_err());
    }
}
