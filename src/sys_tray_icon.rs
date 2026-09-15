//! Native notification-area icon and its small settings menu.
//!
//! The shell owns the visible tray copy, while this value keeps the icon
//! handle and notification registration alive for the same lifetime as the
//! application.  Only right-click/context-menu callbacks are acted upon;
//! double-clicks and every other tray event remain no-ops.

use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::{NonNull, null, null_mut};

use crate::session_window::BackgroundMode;
use crate::switcher_application::AltTabBehavior;
use windows_sys::Win32::Foundation::{GetLastError, HWND, POINT};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::UI::Shell::{
    ExtractAssociatedIconW, NIF_ICON, NIF_MESSAGE, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos, HICON, HMENU,
    IDI_APPLICATION, LoadIconW, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, PostMessageW,
    SetForegroundWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TrackPopupMenu, WM_APP, WM_CONTEXTMENU, WM_NULL, WM_RBUTTONUP,
};

pub const TRAY_CALLBACK_MESSAGE: u32 = WM_APP + 1;

const TRAY_ICON_ID: u32 = 1;
const STICKY_COMMAND: usize = 1;
const TAP_COMMAND: usize = 2;
const FULL_DESKTOP_COMMAND: usize = 3;
const IMAGE_ONLY_COMMAND: usize = 4;
const BLACK_COMMAND: usize = 5;
const EXIT_COMMAND: usize = 6;

/// An action selected from the tray menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayAction {
    Exit,
    SetAltTabBehavior(AltTabBehavior),
    SetBackgroundMode(BackgroundMode),
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
struct PopupMenu(Option<NonNull<c_void>>);

impl PopupMenu {
    fn new() -> Option<Self> {
        NonNull::new(unsafe { CreatePopupMenu() }).map(|handle| Self(Some(handle)))
    }

    fn handle(&self) -> HMENU {
        self.0
            .expect("an attached popup menu is no longer directly usable")
            .as_ptr()
    }

    /// Attach this menu as a child of another menu and transfer ownership to
    /// Win32. `DestroyMenu` on the parent recursively destroys attached
    /// submenus, so the child must no longer run its own `Drop` afterwards.
    fn attach_to(mut self, parent: HMENU, label: &str) -> bool {
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

    /// Process a notification-area callback. Only context/right-click events
    /// open the menu. The settings are copied in before entering the modal
    /// menu loop, so no Rust borrow of the application crosses `TrackPopupMenu`.
    pub fn handle_callback(
        owner: HWND,
        event: isize,
        alt_tab_behavior: AltTabBehavior,
        background_mode: BackgroundMode,
    ) -> Option<TrayAction> {
        match event as u32 {
            WM_RBUTTONUP | WM_CONTEXTMENU => {
                Self::show_menu(owner, alt_tab_behavior, background_mode)
            }
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

    fn show_menu(
        owner: HWND,
        alt_tab_behavior: AltTabBehavior,
        background_mode: BackgroundMode,
    ) -> Option<TrayAction> {
        let menu = PopupMenu::new()?;

        let alt_tab_menu = PopupMenu::new()?;
        if !append_item(
            alt_tab_menu.handle(),
            STICKY_COMMAND,
            "Sticky",
            alt_tab_behavior == AltTabBehavior::Sticky,
        ) || !append_item(
            alt_tab_menu.handle(),
            TAP_COMMAND,
            "Tap (classic)",
            alt_tab_behavior == AltTabBehavior::Tap,
        ) || !alt_tab_menu.attach_to(menu.handle(), "Alt-Tab behavior")
        {
            return None;
        }

        let background_menu = PopupMenu::new()?;
        if !append_item(
            background_menu.handle(),
            FULL_DESKTOP_COMMAND,
            "Full desktop",
            background_mode == BackgroundMode::FullDesktop,
        ) || !append_item(
            background_menu.handle(),
            IMAGE_ONLY_COMMAND,
            "Background image only",
            background_mode == BackgroundMode::ImageOnly,
        ) || !append_item(
            background_menu.handle(),
            BLACK_COMMAND,
            "Black rectangle",
            background_mode == BackgroundMode::Black,
        ) || !background_menu.attach_to(menu.handle(), "Background")
        {
            return None;
        }

        if unsafe { AppendMenuW(menu.handle(), MF_SEPARATOR, 0, null()) } == 0 {
            return None;
        }

        if !append_item(menu.handle(), EXIT_COMMAND, "Exit", false) {
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

        match command {
            STICKY_COMMAND => Some(TrayAction::SetAltTabBehavior(AltTabBehavior::Sticky)),
            TAP_COMMAND => Some(TrayAction::SetAltTabBehavior(AltTabBehavior::Tap)),
            FULL_DESKTOP_COMMAND => {
                Some(TrayAction::SetBackgroundMode(BackgroundMode::FullDesktop))
            }
            IMAGE_ONLY_COMMAND => Some(TrayAction::SetBackgroundMode(BackgroundMode::ImageOnly)),
            BLACK_COMMAND => Some(TrayAction::SetBackgroundMode(BackgroundMode::Black)),
            EXIT_COMMAND => Some(TrayAction::Exit),
            _ => None,
        }
    }
}

impl Drop for SysTrayIcon {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

fn append_item(menu: HMENU, command: usize, label: &str, checked: bool) -> bool {
    let text = wide(label);
    let flags = MF_STRING | if checked { MF_CHECKED } else { 0 };
    unsafe { AppendMenuW(menu, flags, command, text.as_ptr()) != 0 }
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
