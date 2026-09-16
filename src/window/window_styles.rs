//! Window style bits consulted by the window finder.

use windows_sys::Win32::UI::WindowsAndMessaging::{WS_DISABLED, WS_MINIMIZE, WS_VISIBLE};

/// The style bits consulted by the window finder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowStyles(pub i64);

impl WindowStyles {
    pub const DISABLED: Self = Self(WS_DISABLED as i64);
    pub const VISIBLE: Self = Self(WS_VISIBLE as i64);
    pub const MINIMIZE: Self = Self(WS_MINIMIZE as i64);

    pub const fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}
