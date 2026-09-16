//! Ownership wrapper for temporary tray popup menus.

use std::ffi::c_void;
use std::ptr::NonNull;

use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, HMENU, MF_POPUP, MF_STRING,
};

/// Owns a temporary popup menu until it has been dismissed.
pub(super) struct PopupMenu(Option<NonNull<c_void>>);

impl PopupMenu {
    pub(super) fn new() -> Option<Self> {
        NonNull::new(unsafe { CreatePopupMenu() }).map(|handle| Self(Some(handle)))
    }

    pub(super) fn handle(&self) -> HMENU {
        self.0
            .expect("an attached popup menu is no longer directly usable")
            .as_ptr()
    }

    /// Attach this menu as a child of another menu and transfer ownership to
    /// Win32. `DestroyMenu` on the parent recursively destroys attached
    /// submenus, so the child must no longer run its own `Drop` afterwards.
    pub(super) fn attach_to(mut self, parent: HMENU, label: &str) -> bool {
        let text = wide(label);
        let result = unsafe {
            AppendMenuW(
                parent,
                MF_POPUP | MF_STRING,
                self.handle() as usize,
                text.as_ptr(),
            )
        };
        if result == 0 {
            return false;
        }
        // The parent menu now owns this child and will destroy it
        // recursively. Clear the guard before it is dropped so ownership is
        // transferred without leaking or double-destroying the HMENU.
        self.0.take();
        true
    }
}

impl Drop for PopupMenu {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            unsafe {
                DestroyMenu(handle.as_ptr());
            }
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
