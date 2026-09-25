//! Native notification-area icon and its small settings menu.
//!
//! The shell owns the visible tray copy, while this value keeps the icon
//! handle and notification registration alive for the same lifetime as the
//! application.  Only right-click/context-menu callbacks are acted upon;
//! double-clicks and every other tray event remain no-ops.

use std::mem::size_of;
use std::ptr::{null, null_mut};

use super::application_icon::ApplicationIcon;
use super::owned_icon::OwnedIcon;
use super::popup_menu::PopupMenu;

use crate::switcher::{AltTabBehavior, BackgroundMode};
use windows_sys::Win32::Foundation::{GetLastError, HWND, POINT};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::UI::Shell::{
    ExtractAssociatedIconW, NIF_ICON, NIF_MESSAGE, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, GetCursorPos, HMENU, IDI_APPLICATION, LoadIconW, MF_CHECKED, MF_SEPARATOR,
    MF_STRING, PostMessageW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenu, WM_APP, WM_CONTEXTMENU, WM_NULL, WM_RBUTTONUP,
};

pub const TRAY_CALLBACK_MESSAGE: u32 = WM_APP + 1;

const TRAY_ICON_ID: u32 = 1;
const STICKY_COMMAND: usize = 1;
const TAP_COMMAND: usize = 2;
const FULL_DESKTOP_COMMAND: usize = 3;
const IMAGE_ONLY_COMMAND: usize = 4;
const BLACK_COMMAND: usize = 5;
const CLOSE_BUTTONS_COMMAND: usize = 6;
const EXIT_COMMAND: usize = 7;

pub use super::tray_action::TrayAction;

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
        close_buttons_visible: bool,
    ) -> Option<TrayAction> {
        match event as u32 {
            WM_RBUTTONUP | WM_CONTEXTMENU => Self::show_menu(
                owner,
                alt_tab_behavior,
                background_mode,
                close_buttons_visible,
            ),
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
        close_buttons_visible: bool,
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

        if !append_item(
            menu.handle(),
            CLOSE_BUTTONS_COMMAND,
            "Close buttons",
            close_buttons_visible,
        ) {
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
            CLOSE_BUTTONS_COMMAND => {
                Some(TrayAction::SetCloseButtonsVisible(!close_buttons_visible))
            }
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
