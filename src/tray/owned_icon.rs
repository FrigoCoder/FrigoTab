//! Ownership wrapper for extracted shell icons.

use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};

/// Owns an icon returned by an API that transfers ownership to the caller.
///
/// Stock icons returned by `LoadIconW` are intentionally not represented by
/// this type: the system owns those handles and they must not be destroyed.
pub(super) struct OwnedIcon(HICON);

impl OwnedIcon {
    pub(super) fn new(handle: HICON) -> Option<Self> {
        (!handle.is_null()).then_some(Self(handle))
    }

    pub(super) fn handle(&self) -> HICON {
        self.0
    }
}

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        // The handle came from ExtractAssociatedIconW, which transfers icon
        // ownership to the caller.
        unsafe {
            DestroyIcon(self.0);
        }
    }
}
