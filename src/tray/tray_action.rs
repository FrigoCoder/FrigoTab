//! Actions selected from the native tray menu.

use crate::switcher::{AltTabBehavior, BackgroundMode};

/// An action selected from the tray menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayAction {
    Exit,
    SetAltTabBehavior(AltTabBehavior),
    SetBackgroundMode(BackgroundMode),
}
