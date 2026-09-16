//! Owned GDI+ solid brush helper.

use std::ptr::{NonNull, null_mut};

use windows_sys::Win32::Graphics::GdiPlus::{
    GdipCreateSolidFill, GdipDeleteBrush, GpBrush, GpSolidFill,
};

use super::error::{Error, Result, check};

/// Owns a GDI+ solid brush.  Keeping the native handle in a non-null RAII
/// wrapper makes the temporary brush cleanup unconditional, including when a
/// drawing operation is extended with an early return later.
pub(super) struct SolidBrush(NonNull<GpSolidFill>);

impl SolidBrush {
    pub(super) fn new(argb: u32) -> Result<Self> {
        let mut raw = null_mut();
        check(unsafe { GdipCreateSolidFill(argb, &mut raw) })?;
        NonNull::new(raw).map(Self).ok_or(Error(1))
    }

    pub(super) fn as_brush(&self) -> *mut GpBrush {
        self.0.as_ptr().cast()
    }
}

impl Drop for SolidBrush {
    fn drop(&mut self) {
        unsafe {
            let _ = GdipDeleteBrush(self.as_brush());
        }
    }
}
