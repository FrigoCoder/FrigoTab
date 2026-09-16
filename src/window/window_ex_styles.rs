//! Extended window style bits consulted by the window finder.

use windows_sys::Win32::UI::WindowsAndMessaging::{
    WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

/// The extended style bits consulted by the window finder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowExStyles(pub i64);

impl WindowExStyles {
    pub const TRANSPARENT: Self = Self(WS_EX_TRANSPARENT as i64);
    pub const TOOL_WINDOW: Self = Self(WS_EX_TOOLWINDOW as i64);
    pub const APP_WINDOW: Self = Self(WS_EX_APPWINDOW as i64);
    pub const LAYERED: Self = Self(WS_EX_LAYERED as i64);
    pub const NO_ACTIVATE: Self = Self(WS_EX_NOACTIVATE as i64);

    pub const fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}
