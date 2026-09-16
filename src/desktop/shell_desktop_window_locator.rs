use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowExW, GetShellWindow};

pub(super) struct ShellDesktopWindowLocator;

const fn wide_class<const N: usize>(bytes: &[u8; N]) -> [u16; N] {
    let mut wide = [0; N];
    let mut index = 0;
    while index < N {
        wide[index] = bytes[index] as u16;
        index += 1;
    }
    wide
}

const WORKERW_CLASS: [u16; 8] = wide_class(b"WorkerW\0");
const DESKTOP_VIEW_CLASS: [u16; 17] = wide_class(b"SHELLDLL_DefView\0");
const ICON_VIEW_CLASS: [u16; 14] = wide_class(b"SysListView32\0");

impl ShellDesktopWindowLocator {
    pub(super) fn find() -> HWND {
        let shell = unsafe { GetShellWindow() };
        let desktop_view = Self::find_desktop_view(shell);
        if !desktop_view.is_null() {
            return shell;
        }

        let mut worker = null_mut();
        loop {
            worker = unsafe { FindWindowExW(null_mut(), worker, WORKERW_CLASS.as_ptr(), null()) };
            if worker.is_null() {
                return null_mut();
            }
            if !Self::find_desktop_view(worker).is_null() {
                return worker;
            }
        }
    }

    fn find_desktop_view(parent: HWND) -> HWND {
        if parent.is_null() {
            return null_mut();
        }
        let desktop_view =
            unsafe { FindWindowExW(parent, null_mut(), DESKTOP_VIEW_CLASS.as_ptr(), null()) };
        if desktop_view.is_null() {
            return null_mut();
        }

        // Require Explorer's icon list so an unrelated intermediate window
        // with the same class cannot become a capture source.
        let icon_view =
            unsafe { FindWindowExW(desktop_view, null_mut(), ICON_VIEW_CLASS.as_ptr(), null()) };
        if icon_view.is_null() {
            null_mut()
        } else {
            desktop_view
        }
    }
}
