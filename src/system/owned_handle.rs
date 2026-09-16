//! Ownership wrapper for native handles used by system guards.

use std::ptr::NonNull;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};

pub(super) struct OwnedHandle(NonNull<std::ffi::c_void>);

impl OwnedHandle {
    pub(super) fn new(handle: HANDLE) -> Option<Self> {
        NonNull::new(handle).map(Self)
    }

    pub(super) fn raw(&self) -> HANDLE {
        self.0.as_ptr()
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.raw());
        }
    }
}
