use super::screen_point::ScreenPoint;

/// A screen-coordinate rectangle independent of any GUI toolkit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScreenRectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ScreenRectangle {
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

    pub const fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    pub const fn contains(self, point: ScreenPoint) -> bool {
        !self.is_empty()
            && point.x >= self.x
            && point.x < self.right()
            && point.y >= self.y
            && point.y < self.bottom()
    }
}

impl std::hash::Hash for ScreenRectangle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // This is the original unchecked sequence, not Rust's tuple hash.
        let mut hash = self.x;
        hash = hash.wrapping_mul(397) ^ self.y;
        hash = hash.wrapping_mul(397) ^ self.width;
        hash = hash.wrapping_mul(397) ^ self.height;
        state.write_i32(hash);
    }
}

impl std::fmt::Display for ScreenRectangle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "({}, {}, {}, {})",
            self.x, self.y, self.width, self.height
        )
    }
}
