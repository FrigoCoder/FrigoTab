use std::ffi::c_void;
use std::ptr::NonNull;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::{GetDC, HDC, ReleaseDC};

pub(super) struct WindowDc {
    owner: HWND,
    handle: NonNull<c_void>,
}

impl WindowDc {
    pub(super) fn acquire(owner: HWND) -> Option<Self> {
        NonNull::new(unsafe { GetDC(owner) }).map(|handle| Self { owner, handle })
    }

    pub(super) fn handle(&self) -> HDC {
        self.handle.as_ptr()
    }
}

impl Drop for WindowDc {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(self.owner, self.handle());
        }
    }
}
