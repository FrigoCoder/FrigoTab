use windows_sys::Win32::Foundation::RECT;

use crate::desktop::shell_desktop_snapshot::ShellDesktopSnapshot;

use super::background_mode::BackgroundMode;

pub(super) struct BackgroundState {
    pub(super) mode: BackgroundMode,
    pub(super) desktop_snapshot: Option<ShellDesktopSnapshot>,
    pub(super) desktop_snapshot_bounds: RECT,
    pub(super) owner_bounds: RECT,
    pub(super) disposed: bool,
}
