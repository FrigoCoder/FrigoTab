//! A thin value wrapper over an `HWND`.
//!
//! These methods deliberately mirror the native calls made by the original
//! `WindowHandle` struct.  There is no input-queue attachment or synthetic
//! key transition: activation is the historical zero-value `keybd_event`
//! nudge followed by `SetForegroundWindow`.

use std::ffi::c_void;
use std::mem::size_of;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::keybd_event;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWL_STYLE, GetForegroundWindow, GetWindowLongPtrW, GetWindowPlacement,
    GetWindowRect, GetWindowTextLengthW, GetWindowTextW, PostMessageW, SW_RESTORE,
    SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow,
    WINDOWPLACEMENT, WS_DISABLED, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_MINIMIZE, WS_VISIBLE,
};

use crate::rect::{Rect, Rectangle};

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

/// A native window handle with the value semantics of the original wrapper.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WindowHandle(HWND);

impl WindowHandle {
    pub const NULL: Self = Self(std::ptr::null_mut());

    pub const fn new(handle: HWND) -> Self {
        Self(handle)
    }

    pub const fn raw(self) -> HWND {
        self.0
    }

    pub const fn is_null(self) -> bool {
        self.0.is_null()
    }

    pub fn get_foreground_window() -> Self {
        // SAFETY: GetForegroundWindow has no pointer arguments.
        Self(unsafe { GetForegroundWindow() })
    }

    pub fn get_window_styles(self) -> WindowStyles {
        // SAFETY: GetWindowLongPtrW validates the opaque HWND in the same way
        // as the original P/Invoke call.
        WindowStyles(unsafe { GetWindowLongPtrW(self.0, GWL_STYLE) } as i64)
    }

    pub fn get_window_ex_styles(self) -> WindowExStyles {
        WindowExStyles(unsafe { GetWindowLongPtrW(self.0, GWL_EXSTYLE) } as i64)
    }

    /// Post a message and return the native BOOL result.
    pub fn post_message(self, message: u32, wparam: isize, lparam: isize) -> bool {
        // SAFETY: Windows copies the scalar parameters before this call
        // returns; no Rust reference crosses the ABI boundary.
        unsafe { PostMessageW(self.0, message, wparam as usize, lparam) != 0 }
    }

    /// Restore a minimized window and bring it to the foreground.
    ///
    /// The historical zero-value `keybd_event` call is intentional.  It gives
    /// `SetForegroundWindow` recent-input context without joining the target
    /// process's input queue.
    pub fn set_foreground(self) -> bool {
        if self.get_window_styles().contains(WindowStyles::MINIMIZE) {
            // SAFETY: The command and HWND are the same values used by the
            // original P/Invoke call; its return value is intentionally
            // ignored.
            unsafe {
                ShowWindow(self.0, SW_RESTORE);
            }
        }

        // SAFETY: This is the exact no-op keybd_event nudge used by the
        // original application.
        unsafe {
            keybd_event(0, 0, 0, 0);
            SetForegroundWindow(self.0) != 0
        }
    }

    /// Read the Unicode window title.
    pub fn get_window_text(self) -> String {
        // SAFETY: GetWindowTextLengthW has no writable pointer argument.
        let length = unsafe { GetWindowTextLengthW(self.0) };
        if length < 0 {
            return String::new();
        }

        // A wide buffer of length + 1 includes room
        // for the terminator.  Saturating here avoids an integer wrap if a
        // malformed native length is ever returned.
        let capacity = (length as usize).saturating_add(1);
        let mut text = vec![0u16; capacity];
        if capacity == 0 {
            return String::new();
        }
        // SAFETY: The vector is writable for `capacity` UTF-16 units and the
        // native call receives the same count as StringBuilder.Capacity.
        let copied = unsafe {
            GetWindowTextW(
                self.0,
                text.as_mut_ptr(),
                capacity.min(i32::MAX as usize) as i32,
            )
        };
        if copied <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&text[..(copied as usize).min(text.len())])
    }

    /// Return the restored/current screen rectangle, if native geometry is
    /// still valid.  A minimized window uses `rcNormalPosition`; a maximized
    /// window uses the current `GetWindowRect`, exactly as the original code does.
    pub fn try_get_rect(self) -> Option<Rectangle> {
        let mut placement = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        // SAFETY: `placement` is a valid writable WINDOWPLACEMENT whose length
        // is initialized to sizeof(WINDOWPLACEMENT).
        if unsafe { GetWindowPlacement(self.0, &mut placement) } == 0 {
            return None;
        }

        let native = if placement.showCmd == SW_SHOWNORMAL as u32
            || placement.showCmd == SW_SHOWMINIMIZED as u32
        {
            placement.rcNormalPosition
        } else if placement.showCmd == SW_SHOWMAXIMIZED as u32 {
            let mut rect = RECT::default();
            // SAFETY: `rect` is a valid writable RECT.
            if unsafe { GetWindowRect(self.0, &mut rect) } == 0 {
                return None;
            }
            rect
        } else {
            return None;
        };

        let rectangle = Rectangle::new(
            native.left,
            native.top,
            native.right - native.left,
            native.bottom - native.top,
        );
        (rectangle.width > 0 && rectangle.height > 0).then_some(rectangle)
    }

    /// Return a corner-based rectangle, preserving the original `GetRect` shape.
    pub fn get_rect(self) -> Option<Rect> {
        self.try_get_rect().map(Rect::from_rectangle)
    }
}

// HWND values are process-independent kernel/window-manager identities.  The
// wrapper carries no borrowed Rust memory and is safe to copy between the
// small worker/UI threads used by the application.
unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}

// Keep this alias private but make the raw type visible to debuggers and to
// future API additions without changing the value semantics above.
#[allow(dead_code)]
type RawHandle = *mut c_void;
