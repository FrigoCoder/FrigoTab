//! Native notification-area icon and its one-item Exit menu.
//!
//! The shell owns the visible tray copy, while this value keeps the icon
//! handle and notification registration alive for the same lifetime as the
//! application.  Only right-click/context-menu callbacks are acted upon;
//! double-clicks and every other tray event remain no-ops.

use std::mem::size_of;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{GetLastError, HWND, POINT};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::UI::Shell::{
    ExtractAssociatedIconW, NIF_ICON, NIF_MESSAGE, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos, IDI_APPLICATION,
    LoadIconW, MF_STRING, PostMessageW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WM_APP, WM_CONTEXTMENU, WM_NULL, WM_RBUTTONUP,
};

pub const TRAY_CALLBACK_MESSAGE: u32 = WM_APP + 1;

const TRAY_ICON_ID: u32 = 1;
const EXIT_COMMAND: usize = 1;

/// The only action exposed by the tray menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayAction {
    Exit,
}

/// Owns the notification-area registration and the associated icon handle.
pub struct SysTrayIcon {
    owner: HWND,
    data: NOTIFYICONDATAW,
    icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
    destroy_icon: bool,
    added: bool,
}

impl SysTrayIcon {
    /// Register the application's icon with the notification area.
    pub fn new(owner: HWND) -> Result<Self, u32> {
        if owner.is_null() {
            return Err(6); // ERROR_INVALID_HANDLE
        }

        let (icon, destroy_icon) = application_icon();
        let mut data = NOTIFYICONDATAW::default();
        data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = owner;
        data.uID = TRAY_ICON_ID;
        data.uFlags = NIF_MESSAGE | NIF_ICON;
        data.uCallbackMessage = TRAY_CALLBACK_MESSAGE;
        data.hIcon = icon;

        if unsafe { Shell_NotifyIconW(NIM_ADD, &data) } == 0 {
            if destroy_icon && !icon.is_null() {
                unsafe {
                    DestroyIcon(icon);
                }
            }
            return Err(unsafe { GetLastError() });
        }

        Ok(Self {
            owner,
            data,
            icon,
            destroy_icon,
            added: true,
        })
    }

    /// Alias matching the old native integration's construction call.
    pub fn add(owner: HWND) -> Result<Self, u32> {
        Self::new(owner)
    }

    /// Process a notification-area callback.  Only context/right-click
    /// events open the one-item Exit menu.
    pub fn handle_callback(&self, event: isize) -> Option<TrayAction> {
        match event as u32 {
            WM_RBUTTONUP | WM_CONTEXTMENU => self.show_menu(),
            _ => None,
        }
    }

    /// Remove the icon immediately.  Drop performs the same cleanup.
    pub fn remove(&mut self) -> Result<(), u32> {
        if !self.added {
            return Ok(());
        }
        if unsafe { Shell_NotifyIconW(NIM_DELETE, &self.data) } == 0 {
            return Err(unsafe { GetLastError() });
        }
        self.added = false;
        Ok(())
    }

    fn show_menu(&self) -> Option<TrayAction> {
        let menu = unsafe { CreatePopupMenu() };
        if menu.is_null() {
            return None;
        }

        let label = wide("Exit");
        if unsafe { AppendMenuW(menu, MF_STRING, EXIT_COMMAND, label.as_ptr()) } == 0 {
            unsafe {
                DestroyMenu(menu);
            }
            return None;
        }

        let mut point = POINT { x: 0, y: 0 };
        let command = if unsafe { GetCursorPos(&mut point) } != 0 {
            unsafe {
                SetForegroundWindow(self.owner);
                TrackPopupMenu(
                    menu,
                    TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
                    point.x,
                    point.y,
                    0,
                    self.owner,
                    null(),
                ) as usize
            }
        } else {
            0
        };

        unsafe {
            // This mirrors NotifyIcon's normal menu dismissal nudge.
            PostMessageW(self.owner, WM_NULL, 0, 0);
            DestroyMenu(menu);
        }

        (command == EXIT_COMMAND).then_some(TrayAction::Exit)
    }
}

impl Drop for SysTrayIcon {
    fn drop(&mut self) {
        let _ = self.remove();
        if self.destroy_icon && !self.icon.is_null() {
            unsafe {
                DestroyIcon(self.icon);
            }
            self.icon = null_mut();
        }
    }
}

fn application_icon() -> (windows_sys::Win32::UI::WindowsAndMessaging::HICON, bool) {
    // The original Program icon comes from the shell's associated icon lookup
    // for the executable. Use the same shell extraction when the native binary
    // carries an associated icon, then fall back to the stock application
    // icon for an unresource'd development binary.
    let mut path = [0u16; 32768];
    let length = unsafe { GetModuleFileNameW(null_mut(), path.as_mut_ptr(), path.len() as u32) };
    if length > 0 && (length as usize) < path.len() {
        let mut icon_index = 0u16;
        let icon =
            unsafe { ExtractAssociatedIconW(null_mut(), path.as_mut_ptr(), &mut icon_index) };
        if !icon.is_null() {
            return (icon, true);
        }
    }

    (unsafe { LoadIconW(null_mut(), IDI_APPLICATION) }, false)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
