//! Owned GDI+ font and family pair.

use std::ptr::NonNull;
use std::ptr::null_mut;

use windows_sys::Win32::Graphics::GdiPlus::{
    FontStyle, GdipCreateFont, GdipDeleteFont, GpFont, UnitPoint,
};

use super::error::{Error, Result, check};
use super::font_family::FontFamily;
use super::startup::ensure_started;

/// A GDI+ font family and font pair. The family is kept alongside the font
/// because GDI+ fonts borrow it.
pub struct Font {
    pub(super) raw: NonNull<GpFont>,
    _family: FontFamily,
}

impl Font {
    pub fn new(name: &str, size: f32, style: FontStyle) -> Result<Self> {
        ensure_started()?;
        let family = FontFamily::new(name)?;
        let mut raw = null_mut();
        check(unsafe { GdipCreateFont(family.as_ptr(), size, style, UnitPoint, &mut raw) })?;
        let raw = NonNull::new(raw).ok_or(Error(1))?;
        Ok(Self {
            raw,
            _family: family,
        })
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            let _ = GdipDeleteFont(self.raw.as_ptr());
        }
    }
}
