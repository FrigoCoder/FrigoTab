//! Temporary HDC borrowed from a GDI+ graphics object.

use std::ptr::{NonNull, null_mut};

use windows_sys::Win32::Graphics::Gdi::HDC;

use super::error::{Error, Result, check};
use super::graphics::Graphics;

use windows_sys::Win32::Graphics::GdiPlus::GdipGetDC;
use windows_sys::Win32::Graphics::GdiPlus::GdipReleaseDC;

/// A temporary HDC borrowed from a GDI+ graphics object.
///
/// GDI+ requires every successful `GdipGetDC` call to be paired with
/// `GdipReleaseDC` before the graphics object is used again.  Keeping the
/// graphics borrow in this guard prevents the object from being mutably used
/// or dropped while the HDC is outstanding.  The explicit `release` method
/// preserves the original error reporting; `Drop` is the panic/early-return
/// fallback.
pub(super) struct GraphicsDc<'a> {
    graphics: &'a Graphics,
    dc: NonNull<std::ffi::c_void>,
    released: bool,
}

impl<'a> GraphicsDc<'a> {
    pub(super) fn acquire(graphics: &'a Graphics) -> Result<Self> {
        let mut dc = null_mut();
        check(unsafe { GdipGetDC(graphics.raw_ptr(), &mut dc) })?;
        let dc = NonNull::new(dc).ok_or(Error(1))?;
        Ok(Self {
            graphics,
            dc,
            released: false,
        })
    }

    pub(super) fn handle(&self) -> HDC {
        self.dc.as_ptr()
    }

    pub(super) fn release(mut self) -> Result<()> {
        let status = unsafe { GdipReleaseDC(self.graphics.raw_ptr(), self.handle()) };
        self.released = true;
        check(status)
    }
}

impl Drop for GraphicsDc<'_> {
    fn drop(&mut self) {
        if !self.released {
            unsafe {
                let _ = GdipReleaseDC(self.graphics.raw_ptr(), self.handle());
            }
        }
    }
}
