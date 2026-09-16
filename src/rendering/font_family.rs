//! Owned GDI+ font-family handle.

use std::ptr::{NonNull, null_mut};

use windows_sys::Win32::Graphics::GdiPlus::{
    GdipCreateFontFamilyFromName, GdipDeleteFontFamily, GpFontFamily,
};

use super::error::{Error, Result, check};

/// GDI+ fonts borrow their family, so the family is retained by `Font` until
/// the font itself has been released.
pub(super) struct FontFamily(NonNull<GpFontFamily>);

impl FontFamily {
    pub(super) fn new(name: &str) -> Result<Self> {
        let utf16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut raw = null_mut();
        check(unsafe { GdipCreateFontFamilyFromName(utf16.as_ptr(), null_mut(), &mut raw) })?;
        NonNull::new(raw).map(Self).ok_or(Error(1))
    }

    pub(super) fn as_ptr(&self) -> *mut GpFontFamily {
        self.0.as_ptr()
    }
}

impl Drop for FontFamily {
    fn drop(&mut self) {
        unsafe {
            let _ = GdipDeleteFontFamily(self.as_ptr());
        }
    }
}
