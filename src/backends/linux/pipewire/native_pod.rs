// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Source-order, fixed-capacity native SPA negotiation. No Vec-growing builder
//! or variadic C bridge is needed. Actual native comparison remains required.
use super::{Format, spa};
use spa::sys as s;
use std::mem;

#[repr(align(8))]
struct Aligned([u8; 4096]);
pub(super) struct Params {
    bytes: Aligned,
    offsets: [usize; 3],
}
fn checked(result: i32) -> Result<(), ()> {
    if result < 0 { Err(()) } else { Ok(()) }
}
impl Params {
    pub fn new() -> Result<Self, ()> {
        let mut result = Self {
            bytes: Aligned([0; 4096]),
            offsets: [0; 3],
        };
        // SAFETY: Native builder/frame are zero-initializable plain C records.
        // The aligned fixed buffer and each frame stay at stable addresses for
        // all operations. Every builder return is checked; no overflow callback
        // is installed and no pointer into a moved buffer escapes this method.
        unsafe {
            let mut builder: s::spa_pod_builder = mem::zeroed();
            s::spa_pod_builder_init(&mut builder, result.bytes.0.as_mut_ptr().cast(), 4096);
            for (index, modifier) in [true, false].into_iter().enumerate() {
                result.offsets[index] = usize::try_from(builder.state.offset).map_err(|_| ())?;
                format(&mut builder, modifier)?;
            }
            result.offsets[2] = usize::try_from(builder.state.offset).map_err(|_| ())?;
            let mut object = mem::zeroed();
            checked(s::spa_pod_builder_push_object(
                &mut builder,
                &mut object,
                s::SPA_TYPE_OBJECT_ParamBuffers,
                s::SPA_PARAM_Buffers,
            ))?;
            checked(s::spa_pod_builder_prop(
                &mut builder,
                s::SPA_PARAM_BUFFERS_dataType,
                0,
            ))?;
            let mut choice = mem::zeroed();
            checked(s::spa_pod_builder_push_choice(
                &mut builder,
                &mut choice,
                s::SPA_CHOICE_Flags,
                0,
            ))?;
            let types = (1_i32.checked_shl(s::SPA_DATA_MemFd).ok_or(())?)
                | (1_i32.checked_shl(s::SPA_DATA_DmaBuf).ok_or(())?);
            checked(s::spa_pod_builder_int(&mut builder, types))?;
            if s::spa_pod_builder_pop(&mut builder, &mut choice).is_null()
                || s::spa_pod_builder_pop(&mut builder, &mut object).is_null()
            {
                return Err(());
            }
            if usize::try_from(builder.state.offset).map_err(|_| ())? > result.bytes.0.len() {
                return Err(());
            }
        }
        Ok(result)
    }
    pub fn pointers(&self) -> [*const s::spa_pod; 3] {
        self.offsets.map(|offset| {
            // SAFETY: Every offset points at a successfully constructed aligned
            // POD in this owner's initialized fixed buffer. Borrowing self keeps
            // the backing buffer fixed for the caller's synchronous native use.
            unsafe { self.bytes.0.as_ptr().add(offset).cast() }
        })
    }
}

unsafe fn format(builder: *mut s::spa_pod_builder, modifier: bool) -> Result<(), ()> {
    // SAFETY: Caller owns a live fixed-capacity builder. Stack frames stay fixed
    // until their matching pop, and every writing operation checks its result.
    unsafe {
        let mut object = mem::zeroed();
        checked(s::spa_pod_builder_push_object(
            builder,
            &mut object,
            s::SPA_TYPE_OBJECT_Format,
            s::SPA_PARAM_EnumFormat,
        ))?;
        for (key, value) in [
            (s::SPA_FORMAT_mediaType, s::SPA_MEDIA_TYPE_video),
            (s::SPA_FORMAT_mediaSubtype, s::SPA_MEDIA_SUBTYPE_raw),
        ] {
            checked(s::spa_pod_builder_prop(builder, key, 0))?;
            checked(s::spa_pod_builder_id(builder, value))?;
        }
        checked(s::spa_pod_builder_prop(
            builder,
            s::SPA_FORMAT_VIDEO_format,
            0,
        ))?;
        let mut choice = mem::zeroed();
        checked(s::spa_pod_builder_push_choice(
            builder,
            &mut choice,
            s::SPA_CHOICE_Enum,
            0,
        ))?;
        for value in [
            s::SPA_VIDEO_FORMAT_RGB,
            s::SPA_VIDEO_FORMAT_RGBA,
            s::SPA_VIDEO_FORMAT_RGBx,
            s::SPA_VIDEO_FORMAT_BGR,
            s::SPA_VIDEO_FORMAT_BGRA,
            s::SPA_VIDEO_FORMAT_BGRx,
            s::SPA_VIDEO_FORMAT_YUY2,
        ] {
            checked(s::spa_pod_builder_id(builder, value))?;
        }
        if s::spa_pod_builder_pop(builder, &mut choice).is_null() {
            return Err(());
        }
        checked(s::spa_pod_builder_prop(
            builder,
            s::SPA_FORMAT_VIDEO_size,
            0,
        ))?;
        let mut rect = mem::zeroed();
        checked(s::spa_pod_builder_push_choice(
            builder,
            &mut rect,
            s::SPA_CHOICE_Range,
            0,
        ))?;
        for (width, height) in [(1280, 720), (1, 1), (4096, 4096)] {
            checked(s::spa_pod_builder_rectangle(builder, width, height))?;
        }
        if s::spa_pod_builder_pop(builder, &mut rect).is_null() {
            return Err(());
        }
        checked(s::spa_pod_builder_prop(
            builder,
            s::SPA_FORMAT_VIDEO_framerate,
            0,
        ))?;
        let mut fps = mem::zeroed();
        checked(s::spa_pod_builder_push_choice(
            builder,
            &mut fps,
            s::SPA_CHOICE_Range,
            0,
        ))?;
        for (num, denom) in [(30, 1), (0, 1), (1000, 1)] {
            checked(s::spa_pod_builder_fraction(builder, num, denom))?;
        }
        if s::spa_pod_builder_pop(builder, &mut fps).is_null() {
            return Err(());
        }
        if modifier {
            checked(s::spa_pod_builder_prop(
                builder,
                s::SPA_FORMAT_VIDEO_modifier,
                0,
            ))?;
            // DRM_FORMAT_MOD_LINEAR is the defined zero modifier. Its identity
            // and the complete POD bytes still require the native source oracle.
            checked(s::spa_pod_builder_long(builder, 0))?;
        }
        if s::spa_pod_builder_pop(builder, &mut object).is_null() {
            return Err(());
        }
    }
    Ok(())
}

pub(super) fn to_format(value: u32) -> Format {
    match value {
        s::SPA_VIDEO_FORMAT_BGRA => Format::Bgra,
        s::SPA_VIDEO_FORMAT_BGRx => Format::Bgrx,
        s::SPA_VIDEO_FORMAT_RGBA => Format::Rgba,
        s::SPA_VIDEO_FORMAT_RGBx => Format::Rgbx,
        s::SPA_VIDEO_FORMAT_RGB => Format::Rgb,
        s::SPA_VIDEO_FORMAT_BGR => Format::Bgr,
        s::SPA_VIDEO_FORMAT_YUY2 => Format::Yuy2,
        _ => Format::Unknown,
    }
}
fn word(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_ne_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn value(bytes: &[u8], kind: u32, length: usize) -> Option<&[u8]> {
    let (offset, size, actual) = if word(bytes, 4)? == s::SPA_TYPE_Choice {
        (24_usize, word(bytes, 16)?, word(bytes, 20)?)
    } else {
        (8_usize, word(bytes, 0)?, word(bytes, 4)?)
    };
    if actual != kind || usize::try_from(size).ok()? < length {
        return None;
    }
    bytes.get(offset..offset.checked_add(length)?)
}

/// # Safety
/// `param` is a native callback POD covering its declared initialized size for
/// the complete call. It is not retained and no native property pointer escapes.
pub(super) unsafe fn parse(param: *const s::spa_pod) -> Option<(u32, u32, Format)> {
    // SAFETY: Valid callback POD extent is the caller's contract.
    let pod = unsafe { spa::pod::Pod::from_raw(param) };
    let mut info = spa::param::video::VideoInfoRaw::default();
    let (mut media_type, mut media_subtype) = (0, 0);
    // SAFETY: Same valid native POD; outputs are independent initialized scalars.
    if unsafe { s::spa_format_parse(param, &mut media_type, &mut media_subtype) } == 0
        && media_type == s::SPA_MEDIA_TYPE_video
        && media_subtype == s::SPA_MEDIA_SUBTYPE_raw
        && info.parse(pod).is_ok()
    {
        let size = info.size();
        let format = to_format(info.format().as_raw());
        if size.width > 0 && size.height > 0 && format != Format::Unknown {
            return Some((size.width, size.height, format));
        }
    }
    // Exact source fallback accepts the default value in a Choice, including
    // gamescope Choice(None). Bound every property/child read independently.
    let total = usize::try_from(unsafe { (*param).size })
        .ok()?
        .checked_add(mem::size_of::<s::spa_pod>())?;
    isize::try_from(total).ok()?;
    // SAFETY: Callback owns the complete initialized native POD, as above.
    let bytes = unsafe { std::slice::from_raw_parts(param.cast::<u8>(), total) };
    if word(bytes, 4)? != s::SPA_TYPE_Object || total < 16 {
        return None;
    }
    let (mut width, mut height, mut format) = (0, 0, Format::Unknown);
    let mut at = 16_usize;
    while at.checked_add(16)? <= total {
        let key = word(bytes, at)?;
        let start = at.checked_add(8)?;
        let length = usize::try_from(word(bytes, start)?).ok()?.checked_add(8)?;
        let end = start.checked_add(length)?;
        let property = bytes.get(start..end)?;
        if key == s::SPA_FORMAT_VIDEO_size {
            if let Some(rect) = value(property, s::SPA_TYPE_Rectangle, 8) {
                let (w, h) = (word(rect, 0)?, word(rect, 4)?);
                if w > 0 && h > 0 {
                    width = w;
                    height = h;
                }
            }
        } else if key == s::SPA_FORMAT_VIDEO_format
            && let Some(id) = value(property, s::SPA_TYPE_Id, 4)
        {
            format = to_format(word(id, 0)?);
        }
        at = end.checked_add(7)? & !7;
    }
    (width > 0 && height > 0 && format != Format::Unknown).then_some((width, height, format))
}
