//! The small rectangle wrapper used by the Win32 side of FrigoTab.

use crate::points::Point;
use crate::window_handle::WindowHandle;

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

    pub const fn size(self) -> Size {
        Size::new(self.width, self.height)
    }

    pub const fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }
}

/// A width/height pair used by the native layout calculations.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

/// A rectangle kept as two corners, matching the original `Rect` behavior.
///
/// The corners are intentionally private: callers obtain the extent through
/// [`Rect::size`], just as callers of the original type did.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Rect {
    top_left: Point,
    bottom_right: Point,
}

impl Rect {
    pub const fn from_rectangle(bounds: Rectangle) -> Self {
        Self {
            top_left: Point::new(bounds.x, bounds.y),
            bottom_right: Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
        }
    }

    const fn from_points(top_left: Point, bottom_right: Point) -> Self {
        Self {
            top_left,
            bottom_right,
        }
    }

    /// Convert both corners from screen coordinates to client coordinates.
    pub fn screen_to_client(self, window: WindowHandle) -> Self {
        Self::from_points(
            self.top_left.screen_to_client(window),
            self.bottom_right.screen_to_client(window),
        )
    }

    /// Return the extent of the rectangle.
    pub const fn size(self) -> Size {
        Size::new(
            self.bottom_right.x - self.top_left.x,
            self.bottom_right.y - self.top_left.y,
        )
    }

    pub const fn top_left(self) -> Point {
        self.top_left
    }

    pub const fn bottom_right(self) -> Point {
        self.bottom_right
    }
}
