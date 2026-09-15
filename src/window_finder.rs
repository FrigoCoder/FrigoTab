//! The native equivalent of the original `WindowFinder`.
//!
//! Enumeration and filtering intentionally follow the original implementation in
//! the same order.  In particular the root-owner/last-active-popup walk is
//! retained; it is what keeps owned dialog windows from replacing their
//! Alt-Tab owner.

use std::ffi::c_void;

use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::Graphics::Dwm::DwmGetWindowAttribute;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GA_ROOTOWNER, GetAncestor, GetLastActivePopup, IsWindowVisible,
};
use windows_sys::core::BOOL;

use crate::window_handle::{WindowExStyles, WindowHandle, WindowStyles};

const DWMWA_CLOAKED: u32 = 0x0000_000E;

/// Windows eligible for the switcher, in the order returned by EnumWindows.
#[derive(Clone, Debug, Default)]
pub struct WindowFinder {
    pub windows: Vec<WindowHandle>,
}

impl WindowFinder {
    pub fn new() -> Self {
        let mut windows = Vec::new();
        // SAFETY: The callback receives a pointer to this live Vec only for
        // the duration of EnumWindows.  EnumWindows invokes it synchronously.
        unsafe {
            EnumWindows(Some(enum_window_callback), &mut windows as *mut _ as LPARAM);
        }
        Self { windows }
    }
}

unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the pointer supplied by WindowFinder::new and the
    // callback cannot outlive that synchronous EnumWindows invocation.
    let windows = unsafe { &mut *(lparam as *mut Vec<WindowHandle>) };
    let handle = WindowHandle::new(hwnd);
    if get_window_type(handle) == WindowType::AppWindow {
        windows.push(handle);
    }
    1
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WindowType {
    Hidden,
    AppWindow,
}

/// Apply the same visibility/style/Alt-Tab rules as the original
/// `GetWindowType` behavior.
fn get_window_type(handle: WindowHandle) -> WindowType {
    if is_cloaked(handle) {
        return WindowType::Hidden;
    }

    let style = handle.get_window_styles();
    if style.contains(WindowStyles::DISABLED) || !style.contains(WindowStyles::VISIBLE) {
        return WindowType::Hidden;
    }

    let ex_style = handle.get_window_ex_styles();
    if ex_style.contains(WindowExStyles::NO_ACTIVATE) {
        return WindowType::Hidden;
    }
    if ex_style.contains(WindowExStyles::APP_WINDOW) {
        return WindowType::AppWindow;
    }
    if ex_style.contains(WindowExStyles::TOOL_WINDOW) {
        return WindowType::Hidden;
    }

    if is_alt_tab_window(handle) {
        WindowType::AppWindow
    } else {
        WindowType::Hidden
    }
}

fn is_cloaked(window: WindowHandle) -> bool {
    // BOOL is four bytes on Windows.  DwmGetWindowAttribute's return HRESULT
    // is intentionally ignored, matching the original native call.
    let mut cloaked: i32 = 0;
    // SAFETY: `cloaked` is valid for the documented DWM boolean attribute.
    unsafe {
        DwmGetWindowAttribute(
            window.raw(),
            DWMWA_CLOAKED,
            (&mut cloaked as *mut i32).cast::<c_void>(),
            std::mem::size_of::<i32>() as u32,
        );
    }
    cloaked != 0
}

fn is_alt_tab_window(hwnd: WindowHandle) -> bool {
    // `GetAncestor(..., GA_ROOTOWNER)` and the popup walk are intentionally
    // not replaced with a modern heuristic: this is the behavior that the
    // original application shipped.
    let root = unsafe { GetAncestor(hwnd.raw(), GA_ROOTOWNER) };
    get_last_active_visible_popup(WindowHandle::new(root)) == hwnd
}

fn get_last_active_visible_popup(root: WindowHandle) -> WindowHandle {
    let mut hwnd_walk = WindowHandle::NULL;
    let mut hwnd_try = root;
    while hwnd_walk != hwnd_try {
        hwnd_walk = hwnd_try;
        let next = unsafe { GetLastActivePopup(hwnd_walk.raw()) };
        hwnd_try = WindowHandle::new(next);
        if unsafe { IsWindowVisible(hwnd_try.raw()) } != 0 {
            return hwnd_try;
        }
    }
    WindowHandle::NULL
}
