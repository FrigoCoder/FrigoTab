use std::{ffi::c_void, ptr::NonNull};

use windows_sys::Win32::Graphics::Gdi::DeleteObject;

/// `GetIconInfo` transfers ownership of both returned bitmap handles.
pub(super) struct IconInfoBitmaps {
    pub(super) color: Option<NonNull<c_void>>,
    pub(super) mask: Option<NonNull<c_void>>,
}

impl Drop for IconInfoBitmaps {
    fn drop(&mut self) {
        for bitmap in [self.color, self.mask].into_iter().flatten() {
            unsafe {
                DeleteObject(bitmap.as_ptr());
            }
        }
    }
}
