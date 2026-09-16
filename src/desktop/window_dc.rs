use std::ffi::c_void;
use std::ptr::NonNull;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::{GetDC, HDC, ReleaseDC};

use super::diagnostics::write_diagnostic;

/// A DC acquired from a window.  Unlike a memory DC, this handle must be
/// released with ReleaseDC and the owning HWND is therefore retained here.
pub(super) struct WindowDc {
    hwnd: HWND,
    dc: NonNull<c_void>,
}

impl WindowDc {
    pub(super) fn new(hwnd: HWND) -> Result<Self, ()> {
        let dc = unsafe { GetDC(hwnd) };
        let Some(dc) = NonNull::new(dc) else {
            write_diagnostic("GetDC failed for the Explorer desktop host");
            return Err(());
        };
        Ok(Self { hwnd, dc })
    }

    pub(super) fn dc(&self) -> HDC {
        self.dc.as_ptr()
    }
}

impl Drop for WindowDc {
    fn drop(&mut self) {
        if unsafe { ReleaseDC(self.hwnd, self.dc()) } == 0 {
            write_diagnostic("ReleaseDC failed for the Explorer desktop host");
        }
    }
}
