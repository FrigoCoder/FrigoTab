//! A thin value wrapper over an `HWND`.
//!
//! These methods deliberately mirror the native calls made by the original
//! `WindowHandle` struct. There is no input-queue attachment: activation uses
//! the historical zero-value `keybd_event` nudge followed by
//! `SetForegroundWindow`. FrigoTab tags and consumes that no-op transition in
//! its own hook so it cannot become an application keyboard message.

use std::mem::size_of;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::keybd_event;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, GetWindowPlacement, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, IsWindow, SW_RESTORE, SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED,
    SW_SHOWNORMAL, SetForegroundWindow, ShowWindow, WINDOWPLACEMENT,
};

use crate::geometry::Rectangle;

// Keep the style wrappers available through the original `window_handle`
// module path as well as their dedicated modules.
pub use super::window_ex_styles::WindowExStyles;
pub use super::window_styles::WindowStyles;

/// `dwExtraInfo` tag carried by FrigoTab's zero-key foreground-activation
/// nudge. The nudge exists solely to give `SetForegroundWindow` recent-input
/// context; it must not become a keyboard message in the previous foreground
/// application. This is not a security boundary: a same-value injected event
/// from another process is indistinguishable from our own at this layer.
pub(crate) const FOREGROUND_NUDGE_EXTRA_INFO: usize = 0x4652_4e47;

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

    pub fn is_valid(self) -> bool {
        !self.0.is_null() && unsafe { IsWindow(self.0) != 0 }
    }

    pub fn get_window_styles(self) -> WindowStyles {
        // SAFETY: GetWindowLongPtrW validates the opaque HWND in the same way
        // as the original P/Invoke call.
        WindowStyles(unsafe { GetWindowLongPtrW(self.0, GWL_STYLE) } as i64)
    }

    pub fn get_window_ex_styles(self) -> WindowExStyles {
        WindowExStyles(unsafe { GetWindowLongPtrW(self.0, GWL_EXSTYLE) } as i64)
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

        // SAFETY: This is the no-op keybd_event nudge used by the original
        // application. The private marker lets FrigoTab's global hook consume
        // this event before it can become a keyboard message in the previous
        // foreground application.
        unsafe {
            keybd_event(0, 0, 0, FOREGROUND_NUDGE_EXTRA_INFO);
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
}
