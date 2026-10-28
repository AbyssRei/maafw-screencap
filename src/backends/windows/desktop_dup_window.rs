// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from fixed DesktopDupWindowScreencap.{h,cpp}; no P2 geometry fix.
use super::{desktop_dup, pixels::BgraConverter};
use crate::platform::windows::{
    foreground, geometry,
    window_api::{Point, SystemWindow, Window, WindowApi},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Crop {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
pub(crate) trait Converter: BgraConverter {
    fn dimensions(&self, image: &Self::Image) -> (i32, i32);
    // Isolate the logical BGR ROI into independent packed target storage. This
    // explicit safety adaptation adds a target BGR copy after full conversion.
    fn crop_bgr(&mut self, image: &Self::Image, crop: Crop) -> Result<Self::Image, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum Error<E> {
    NullWindow,
    Desktop(desktop_dup::Error<E>),
    Native(desktop_dup::CaptureError),
    Win32(&'static str, u32),
    InvalidClient,
    InvalidOutput,
    OutOfBounds,
    Overflow,
    Conversion(E),
}
pub(crate) struct Coordination {
    pub foreground: bool,
    pub geometry: Result<(), geometry::Error>,
}
pub(crate) struct Attempt<T, E> {
    pub image: Result<T, Error<E>>,
    pub evidence: desktop_dup::Evidence,
    pub coordination: Option<Coordination>,
    pub target_copy_bytes: usize,
}
pub(crate) struct DesktopDupWindowCapture {
    window: Window,
    inner: desktop_dup::DesktopDupCapture,
}
impl DesktopDupWindowCapture {
    pub fn new(window: Window) -> Self {
        Self {
            window,
            inner: desktop_dup::DesktopDupCapture::new(window),
        }
    }
    pub fn capture<C: Converter>(&mut self, converter: &mut C) -> Attempt<C::Image, C::Error> {
        if self.window.is_null() {
            return Attempt {
                image: Err(Error::NullWindow),
                evidence: Default::default(),
                coordination: None,
                target_copy_bytes: 0,
            };
        }
        let coordination = Some(Coordination {
            foreground: foreground::ensure_foreground(self.window),
            geometry: geometry::ensure_on_screen(&SystemWindow, self.window),
        });
        let captured = self.inner.capture(converter);
        let mut target_copy_bytes = 0;
        let image = captured.image.map_err(Error::Desktop).and_then(|image| {
            // Source queries geometry only AFTER full-monitor BGR conversion.
            let client = SystemWindow
                .client_rect(self.window)
                .map_err(|code| Error::Win32("GetClientRect", code))?;
            let origin = SystemWindow
                .origin(
                    self.window,
                    Point {
                        x: client.left,
                        y: client.top,
                    },
                )
                .map_err(|code| Error::Win32("ClientToScreen", code))?;
            let width = client
                .right
                .checked_sub(client.left)
                .ok_or(Error::Overflow)?;
            let height = client
                .bottom
                .checked_sub(client.top)
                .ok_or(Error::Overflow)?;
            let right = origin.x.checked_add(width).ok_or(Error::Overflow)?;
            let bottom = origin.y.checked_add(height).ok_or(Error::Overflow)?;
            if right <= origin.x || bottom <= origin.y {
                return Err(Error::InvalidClient);
            }
            let output = self.inner.output_coordinates().map_err(Error::Native)?;
            if output[2] <= output[0] || output[3] <= output[1] {
                return Err(Error::InvalidOutput);
            }
            let crop = crop_rect(origin, width, height, output, converter.dimensions(&image))?;
            let bytes = usize::try_from(crop.width)
                .map_err(|_| Error::Overflow)?
                .checked_mul(usize::try_from(crop.height).map_err(|_| Error::Overflow)?)
                .and_then(|n| n.checked_mul(3))
                .ok_or(Error::Overflow)?;
            let result = converter
                .crop_bgr(&image, crop)
                .map_err(Error::Conversion)?;
            target_copy_bytes = bytes;
            Ok(result)
        });
        Attempt {
            image,
            evidence: captured.evidence,
            coordination,
            target_copy_bytes,
        }
    }
}
fn crop_rect<E>(
    origin: Point,
    width: i32,
    height: i32,
    output: [i32; 4],
    image: (i32, i32),
) -> Result<Crop, Error<E>> {
    if width <= 0 || height <= 0 {
        return Err(Error::InvalidClient);
    }
    let x = origin.x.checked_sub(output[0]).ok_or(Error::Overflow)?;
    let y = origin.y.checked_sub(output[1]).ok_or(Error::Overflow)?;
    if x < 0
        || y < 0
        || x.checked_add(width).ok_or(Error::Overflow)? > image.0
        || y.checked_add(height).ok_or(Error::Overflow)? > image.1
    {
        return Err(Error::OutOfBounds);
    }
    Ok(Crop {
        x,
        y,
        width,
        height,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_cross_output_regions_and_checks_negative_monitor_origins() {
        assert_eq!(
            crop_rect::<()>(
                Point { x: -1900, y: 50 },
                96,
                64,
                [-1920, 0, 0, 1080],
                (1920, 1080)
            )
            .unwrap(),
            Crop {
                x: 20,
                y: 50,
                width: 96,
                height: 64
            }
        );
        assert!(matches!(
            crop_rect::<()>(
                Point { x: -20, y: 0 },
                96,
                64,
                [0, 0, 1920, 1080],
                (1920, 1080)
            ),
            Err(Error::OutOfBounds)
        ));
        assert!(matches!(
            crop_rect::<()>(
                Point { x: i32::MAX, y: 0 },
                96,
                64,
                [-1, 0, 1920, 1080],
                (1920, 1080)
            ),
            Err(Error::Overflow)
        ));
    }
}
