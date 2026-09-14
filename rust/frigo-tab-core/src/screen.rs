/// A screen-coordinate point.  Coordinates may be negative on a multi-monitor
/// desktop whose primary display is not the top-left monitor.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

impl ScreenPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl std::fmt::Display for ScreenPoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "({}, {})", self.x, self.y)
    }
}

/// A screen-coordinate rectangle independent of any GUI toolkit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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

impl std::fmt::Display for ScreenRectangle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "({}, {}, {}, {})",
            self.x, self.y, self.width, self.height
        )
    }
}
