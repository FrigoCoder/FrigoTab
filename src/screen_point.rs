use std::fmt;

/// A point in virtual-screen coordinates.
///
/// This is deliberately independent of Win32 or a GUI toolkit.  The native
/// adapter translates its mouse coordinates into this type before handing the
/// event to the switcher state machine.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

impl ScreenPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl fmt::Display for ScreenPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
