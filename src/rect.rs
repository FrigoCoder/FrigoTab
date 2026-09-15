//! The small rectangle value used by the Win32 side of FrigoTab.

/// A screen/client rectangle represented by its origin and extent.
///
/// This is the native rectangle value that crosses the UI/native boundary.
/// Coordinates may be negative on a monitor arranged to the left or above the
/// primary display.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rectangle {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn right(self) -> i32 {
        self.x + self.width
    }

    pub const fn bottom(self) -> i32 {
        self.y + self.height
    }
}
