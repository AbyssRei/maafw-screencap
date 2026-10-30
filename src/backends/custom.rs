// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Private original custom screenshot manager slice. No SDK provider ABI.
//! The boundary supplies the actual temporary image buffer and Mat-owner copy
//! in explicit acceptance builds; the core has no OpenCV or C++ dependency.
pub(crate) type Screenshot<B> =
    fn(&B, &mut <B as Boundary>::Buffer) -> Result<u8, <B as Boundary>::Error>;

pub(crate) trait Boundary: Sized {
    type Buffer;
    type Image;
    type Error;
    fn connect(&self) -> Option<Result<u8, Self::Error>> {
        None
    }
    fn connected(&self) -> Option<Result<u8, Self::Error>> {
        None
    }
    fn inactive(&self) -> Option<Result<u8, Self::Error>> {
        None
    }
    // Query the current borrowed table, never copy it at manager construction.
    fn screenshot(&self) -> Option<Screenshot<Self>>;
    // The native boundary constructs the real C++ ImageBuffer on its stack,
    // invokes action once synchronously, then destroys that buffer. No native
    // pointer/lease escapes. A returned Image retains its actual pixel owner.
    fn with_buffer<T>(
        &self,
        action: impl FnOnce(&mut Self::Buffer) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error>;
    fn image(&self, buffer: &Self::Buffer) -> Result<Self::Image, Self::Error>;
    // The explicit native value adapter can assign an actual borrowed C++
    // caller destination here. It must finish before the temporary buffer ends.
    fn publish_image(
        &self,
        output: &mut Self::Image,
        image: Self::Image,
    ) -> Result<(), Self::Error> {
        *output = image;
        Ok(())
    }
}

pub(crate) type UuidCallback<B> =
    fn(&B, &mut <B as IdentityBoundary>::UuidBuffer) -> Result<u8, <B as Boundary>::Error>;

/// The original manager's byte-string identity and feature queries. Kept
/// separate from image storage; neither operation invokes an input action.
pub(crate) trait IdentityBoundary: Boundary {
    type UuidBuffer;
    type Uuid;
    fn uuid_callback(&self) -> Option<UuidCallback<Self>>;
    fn with_uuid_buffer<T>(
        &self,
        action: impl FnOnce(&mut Self::UuidBuffer) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error>;
    fn uuid(&self, buffer: &Self::UuidBuffer) -> Result<Self::Uuid, Self::Error>;
    fn publish_uuid(
        &self,
        output: &mut Self::Uuid,
        uuid: Self::Uuid,
        _buffer: &Self::UuidBuffer,
    ) -> Result<(), Self::Error> {
        *output = uuid;
        Ok(())
    }
    fn features(&self) -> Option<Result<u64, Self::Error>>;
}

pub(crate) type InfoCallback<B> =
    fn(&B, &mut <B as InfoBoundary>::InfoBuffer) -> Result<u8, <B as Boundary>::Error>;

/// Callback access remains on the same borrowed manager. The explicit value
/// adapter retains the source parser's byte, number and duplicate-key rules.
pub(crate) trait InfoBoundary: Boundary {
    type InfoBuffer;
    fn info_callback(&self) -> Option<InfoCallback<Self>>;
    fn with_info_buffer<T>(
        &self,
        action: impl FnOnce(&mut Self::InfoBuffer) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error>;
}

/// Private source-value operations, supplied even when the callback table is
/// absent. This does not select a public JSON representation or core dependency.
pub(crate) trait InfoObjects {
    type Buffer;
    type Info;
    type Error;
    fn empty_object(&self) -> Result<Self::Info, Self::Error>;
    // Copy an independent object while the real temporary StringBuffer lives.
    // Invalid JSON and non-object values return None; exceptions remain errors.
    fn parse_object(&self, buffer: &Self::Buffer) -> Result<Option<Self::Info>, Self::Error>;
    fn set_custom_type(&self, info: &mut Self::Info) -> Result<(), Self::Error>;
}

pub(crate) struct Manager<'a, B> {
    boundary: Option<&'a B>,
}

#[cfg(all(
    test,
    feature = "custom-capture",
    any(
        all(target_os = "windows", feature = "prototype-windows"),
        target_os = "linux",
        target_os = "macos"
    )
))]
#[path = "../../tests/upstream-parity/custom-capture/native.rs"]
mod native_tests;
impl<'a, B: Boundary> Manager<'a, B> {
    // Original public create rejects a null callback table before allocating
    // its manager. Individual optional callbacks may still all be absent.
    pub fn create(boundary: Option<&'a B>) -> Option<Self> {
        boundary.map(|boundary| Self::new(Some(boundary)))
    }

    pub fn new(boundary: Option<&'a B>) -> Self {
        Self { boundary }
    }
    pub fn connect(&self) -> Result<bool, B::Error> {
        self.boundary
            .and_then(B::connect)
            .map_or(Ok(false), |r| r.map(|v| v != 0))
    }
    pub fn connected(&self) -> Result<bool, B::Error> {
        self.boundary
            .and_then(B::connected)
            .map_or(Ok(true), |r| r.map(|v| v != 0))
    }
    pub fn inactive(&self) -> Result<bool, B::Error> {
        self.boundary
            .and_then(B::inactive)
            .map_or(Ok(true), |r| r.map(|v| v != 0))
    }
    pub fn capture(&self, output: &mut B::Image) -> Result<bool, B::Error> {
        let Some(boundary) = self.boundary else {
            return Ok(false);
        };
        let Some(callback) = boundary.screenshot() else {
            return Ok(false);
        };
        boundary.with_buffer(|buffer| {
            if callback(boundary, buffer)? == 0 {
                return Ok(false);
            }
            // Source accepts true+empty too and has no connected-state gate.
            // Extract before releasing the temporary buffer, and replace the
            // caller only after the new real image owner has been obtained.
            boundary.publish_image(output, boundary.image(buffer)?)?;
            Ok(true)
        })
    }
}

impl<B: IdentityBoundary> Manager<'_, B> {
    pub fn request_uuid(&self, output: &mut B::Uuid) -> Result<bool, B::Error> {
        let Some(boundary) = self.boundary else {
            return Ok(false);
        };
        let Some(callback) = boundary.uuid_callback() else {
            return Ok(false);
        };
        boundary.with_uuid_buffer(|buffer| {
            if callback(boundary, buffer)? == 0 {
                return Ok(false);
            }
            // Source copies the complete std::string before its stack buffer
            // ends. A true empty UUID replaces output; false preserves it.
            boundary.publish_uuid(output, boundary.uuid(buffer)?, buffer)?;
            Ok(true)
        })
    }

    pub fn get_features(&self) -> Result<u64, B::Error> {
        self.boundary.and_then(B::features).unwrap_or(Ok(0))
    }
}

impl<B: InfoBoundary> Manager<'_, B> {
    pub fn get_info<V>(&self, values: &V) -> Result<V::Info, B::Error>
    where
        V: InfoObjects<Buffer = B::InfoBuffer, Error = B::Error>,
    {
        let mut info = values.empty_object()?;
        if let Some(boundary) = self.boundary
            && let Some(callback) = boundary.info_callback()
            && let Some(extra) = boundary.with_info_buffer(|buffer| {
                if callback(boundary, buffer)? == 0 {
                    return Ok(None);
                }
                values.parse_object(buffer)
            })?
        {
            info = extra;
        }
        // Source ends get_info_from_controller's stack buffer before replacing
        // type, including when parsing supplied an independently owned object.
        values.set_custom_type(&mut info)?;
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        sync::{Arc, Weak},
    };
    type Image = Option<Arc<[u8; 3]>>;
    struct Buffer<'a> {
        image: Image,
        events: &'a RefCell<Vec<&'static str>>,
    }
    impl Drop for Buffer<'_> {
        fn drop(&mut self) {
            self.events.borrow_mut().push("drop");
        }
    }
    struct Fake {
        present: Cell<bool>,
        result: Cell<Result<u8, &'static str>>,
        populate: Cell<bool>,
        events: RefCell<Vec<&'static str>>,
        temporary: RefCell<Weak<[u8; 3]>>,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                present: Cell::new(true),
                result: Cell::new(Ok(1)),
                populate: Cell::new(true),
                events: RefCell::new(Vec::new()),
                temporary: RefCell::new(Weak::new()),
            }
        }
        fn shot(&self, buffer: &mut Image) -> Result<u8, &'static str> {
            self.events.borrow_mut().push("callback");
            if self.populate.get() {
                let owner = Arc::try_new([3, 7, 11]).unwrap();
                *self.temporary.borrow_mut() = Arc::downgrade(&owner);
                *buffer = Some(owner);
            }
            self.result.get()
        }
    }
    impl Boundary for Fake {
        type Buffer = Image;
        type Image = Image;
        type Error = &'static str;
        fn screenshot(&self) -> Option<Screenshot<Self>> {
            self.present.get().then_some(Self::shot)
        }
        fn with_buffer<T>(
            &self,
            action: impl FnOnce(&mut Image) -> Result<T, Self::Error>,
        ) -> Result<T, Self::Error> {
            self.events.borrow_mut().push("create");
            let mut buffer = Buffer {
                image: None,
                events: &self.events,
            };
            action(&mut buffer.image)
        }
        fn image(&self, buffer: &Image) -> Result<Image, Self::Error> {
            self.events.borrow_mut().push("extract");
            Ok(buffer.clone())
        }
    }
    #[test]
    fn missing_callbacks_preserve_output_without_constructing_a_buffer() {
        let fake = Fake::new();
        let old = Arc::try_new([1, 2, 3]).unwrap();
        let mut output = Some(old.clone());
        for boundary in [None, Some(&fake)] {
            fake.present.set(false);
            let manager = Manager::new(boundary);
            assert!(!manager.connect().unwrap());
            assert!(manager.connected().unwrap() && manager.inactive().unwrap());
            assert!(!manager.capture(&mut output).unwrap());
            assert!(Arc::ptr_eq(output.as_ref().unwrap(), &old));
            assert!(fake.events.borrow().is_empty());
        }
    }
    #[test]
    fn publishes_nonzero_and_empty_results_before_releasing_the_temporary_buffer() {
        for code in [1, 2, 255] {
            for populate in [false, true] {
                let fake = Fake::new();
                fake.result.set(Ok(code));
                fake.populate.set(populate);
                let mut output = Some(Arc::try_new([0, 0, 0]).unwrap());
                assert!(Manager::new(Some(&fake)).capture(&mut output).unwrap());
                assert_eq!(
                    fake.events.borrow().as_slice(),
                    ["create", "callback", "extract", "drop"]
                );
                if populate {
                    assert_eq!(output.as_deref(), Some(&[3, 7, 11]));
                    assert_eq!(Arc::strong_count(output.as_ref().unwrap()), 1);
                } else {
                    assert!(output.is_none());
                }
                drop(fake);
                if populate {
                    assert_eq!(output.as_deref(), Some(&[3, 7, 11]));
                }
            }
        }
    }
    #[test]
    fn false_and_error_release_temporary_pixels_and_preserve_the_callers_owner() {
        for result in [Ok(0), Err("callback failure")] {
            let fake = Fake::new();
            fake.result.set(result);
            let old = Arc::try_new([1, 2, 3]).unwrap();
            let mut output = Some(old.clone());
            assert_eq!(
                Manager::new(Some(&fake)).capture(&mut output),
                result.map(|v| v != 0)
            );
            assert!(Arc::ptr_eq(output.as_ref().unwrap(), &old));
            assert!(fake.temporary.borrow().upgrade().is_none());
            assert_eq!(
                fake.events.borrow().as_slice(),
                ["create", "callback", "drop"]
            );
        }
    }
    #[test]
    fn observes_changes_in_the_borrowed_table_between_calls() {
        let fake = Rc::new(Fake::new());
        let manager = Manager::new(Some(fake.as_ref()));
        let mut output = None;
        assert!(manager.capture(&mut output).unwrap());
        fake.present.set(false);
        let retained = output.as_ref().unwrap().clone();
        assert!(!manager.capture(&mut output).unwrap());
        assert!(Arc::ptr_eq(&retained, output.as_ref().unwrap()));
        fake.present.set(true);
        fake.populate.set(false);
        assert!(manager.capture(&mut output).unwrap());
        assert!(output.is_none());
    }
}
