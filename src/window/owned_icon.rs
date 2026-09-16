use std::ptr::NonNull;

use windows_sys::Win32::UI::WindowsAndMessaging::{CopyIcon, DestroyIcon, HICON};

/// A private owner for icons returned by `CopyIcon` and
/// `ExtractAssociatedIconW`. Handles returned by `LoadIconW` and class-icon
/// lookups are deliberately never wrapped because they are shared.
pub(super) struct OwnedIcon(NonNull<std::ffi::c_void>);

impl OwnedIcon {
    pub(super) fn copy(icon: HICON) -> Result<Self, String> {
        let copied = unsafe { CopyIcon(icon) };
        Self::from_raw(copied).ok_or_else(|| "CopyIcon returned a null icon".to_string())
    }

    pub(super) fn from_raw(icon: HICON) -> Option<Self> {
        NonNull::new(icon).map(Self)
    }

    pub(super) fn raw(&self) -> HICON {
        self.0.as_ptr()
    }
}

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        unsafe {
            DestroyIcon(self.raw());
        }
    }
}
