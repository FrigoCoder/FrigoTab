//! Icon handle variants used by the tray registration.

use super::owned_icon::OwnedIcon;
use windows_sys::Win32::UI::WindowsAndMessaging::HICON;

pub(super) enum ApplicationIcon {
    Owned(OwnedIcon),
    Stock(HICON),
}

impl ApplicationIcon {
    pub(super) fn handle(&self) -> HICON {
        match self {
            Self::Owned(icon) => icon.handle(),
            Self::Stock(handle) => *handle,
        }
    }
}
