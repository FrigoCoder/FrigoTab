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
    AppendMenuW, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos, HICON, HMENU,
    IDI_APPLICATION, LoadIconW, MF_STRING, PostMessageW, SetForegroundWindow, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WM_APP, WM_CONTEXTMENU, WM_NULL,
    WM_RBUTTONUP,
};

pub const TRAY_CALLBACK_MESSAGE: u32 = WM_APP + 1;

const TRAY_ICON_ID: u32 = 1;
const EXIT_COMMAND: usize = 1;

/// The only action exposed by the tray menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayAction {
    Exit,
}

/// Owns an icon returned by an API that transfers ownership to the caller.
///
/// Stock icons returned by `LoadIconW` are intentionally not represented by
/// this type: the system owns those handles and they must not be destroyed.
struct OwnedIcon(HICON);

impl OwnedIcon {
    fn new(handle: HICON) -> Option<Self> {
        (!handle.is_null()).then_some(Self(handle))
    }

    fn handle(&self) -> HICON {
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

enum ApplicationIcon {
    Owned(OwnedIcon),
    Stock(HICON),
}

impl ApplicationIcon {
    fn handle(&self) -> HICON {
        match self {
            Self::Owned(icon) => icon.handle(),
            Self::Stock(handle) => *handle,
        }
    }
}

/// Owns a temporary popup menu until it has been dismissed.
struct PopupMenu(HMENU);

impl PopupMenu {
    fn new() -> Option<Self> {
        let handle = unsafe { CreatePopupMenu() };
        (!handle.is_null()).then_some(Self(handle))
    }

    fn handle(&self) -> HMENU {
        self.0
    }
}

impl Drop for PopupMenu {
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.0);
        }
    }
}

/// Owns the notification-area registration and the associated icon handle.
pub struct SysTrayIcon {
    data: NOTIFYICONDATAW,
    // Kept solely for ownership; dropping it releases an extracted icon.
    _icon: ApplicationIcon,
    added: bool,
}

impl SysTrayIcon {
    /// Register the application's icon with the notification area.
    pub fn new(owner: HWND) -> Result<Self, u32> {
        if owner.is_null() {
            return Err(6); // ERROR_INVALID_HANDLE
        }

        let icon = application_icon();
        let data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: owner,
            uID: TRAY_ICON_ID,
            uFlags: NIF_MESSAGE | NIF_ICON,
            uCallbackMessage: TRAY_CALLBACK_MESSAGE,
            hIcon: icon.handle(),
            ..Default::default()
        };

        if unsafe { Shell_NotifyIconW(NIM_ADD, &data) } == 0 {
            return Err(unsafe { GetLastError() });
        }

        Ok(Self {
            data,
            _icon: icon,
            added: true,
        })
    }

    /// Process a notification-area callback.  Only context/right-click
    /// events open the one-item Exit menu.
    pub fn handle_callback(owner: HWND, event: isize) -> Option<TrayAction> {
        match event as u32 {
            WM_RBUTTONUP | WM_CONTEXTMENU => Self::show_menu(owner),
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

    fn show_menu(owner: HWND) -> Option<TrayAction> {
        let menu = PopupMenu::new()?;

        let label = wide("Exit");
        if unsafe { AppendMenuW(menu.handle(), MF_STRING, EXIT_COMMAND, label.as_ptr()) } == 0 {
            return None;
        }

        let mut point = POINT { x: 0, y: 0 };
        let command = if unsafe { GetCursorPos(&mut point) } != 0 {
            unsafe {
                SetForegroundWindow(owner);
                TrackPopupMenu(
                    menu.handle(),
                    TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
                    point.x,
                    point.y,
                    0,
                    owner,
                    null(),
                ) as usize
            }
        } else {
            0
        };

        unsafe {
            // This mirrors NotifyIcon's normal menu dismissal nudge.
            PostMessageW(owner, WM_NULL, 0, 0);
        }

        (command == EXIT_COMMAND).then_some(TrayAction::Exit)
    }
}

impl Drop for SysTrayIcon {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

fn application_icon() -> ApplicationIcon {
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
        if let Some(icon) = OwnedIcon::new(icon) {
            return ApplicationIcon::Owned(icon);
        }
    }

    ApplicationIcon::Stock(unsafe { LoadIconW(null_mut(), IDI_APPLICATION) })
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
