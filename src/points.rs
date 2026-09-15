//! The two coordinate conversions used by the original FrigoTab code.
//!
//! The screen point is only a small value type in the original application;
//! this is the equivalent value type for the native implementation. The
//! native functions intentionally retain the original failure semantics: their
//! return value is ignored and the (possibly
//! unchanged) point is returned to the caller.

use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};

use crate::window_handle::WindowHandle;

/// A point in either client or screen coordinates.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Convert a client point to screen coordinates.
    pub fn client_to_screen(self, handle: WindowHandle) -> Self {
        let mut point = POINT {
            x: self.x,
            y: self.y,
        };
        // Deliberately do not branch on BOOL. Keep that behavior: a failed
        // conversion returns the native value as it
        // was supplied.
        unsafe {
            ClientToScreen(handle.raw(), &mut point);
        }
        Self::new(point.x, point.y)
    }

    /// Convert a screen point to client coordinates.
    pub fn screen_to_client(self, handle: WindowHandle) -> Self {
        let mut point = POINT {
            x: self.x,
            y: self.y,
        };
        unsafe {
            ScreenToClient(handle.raw(), &mut point);
        }
        Self::new(point.x, point.y)
    }
}
