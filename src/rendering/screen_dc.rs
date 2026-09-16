//! Long-lived screen device context for a layered surface.

use std::ffi::c_void;
use std::ptr::{NonNull, null_mut};

use windows_sys::Win32::Graphics::Gdi::{GetDC, HDC, ReleaseDC};

use super::layer_errors::win32_error;

/// A screen DC kept alive for the lifetime of its layer surface.
///
/// `GetDC(null_mut())` obtains a DC for the entire screen. The updater keeps
/// that DC instead of reacquiring it for every redraw, matching the original
/// implementation's resource lifetime.
pub(super) struct ScreenDc {
    handle: NonNull<c_void>,
}

impl ScreenDc {
    pub(super) fn acquire() -> Result<Self, String> {
        let handle = unsafe { GetDC(null_mut()) };
        NonNull::new(handle)
            .map(|handle| Self { handle })
            .ok_or_else(|| win32_error("GetDC"))
    }

    pub(super) fn handle(&self) -> HDC {
        self.handle.as_ptr()
    }
}

impl Drop for ScreenDc {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(null_mut(), self.handle());
        }
    }
}
