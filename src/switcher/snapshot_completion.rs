use windows_sys::Win32::Foundation::RECT;

use crate::desktop::shell_desktop_snapshot::ShellDesktopSnapshot;

pub(super) struct SnapshotCompletion {
    pub(super) snapshot: ShellDesktopSnapshot,
    pub(super) bounds: RECT,
}
