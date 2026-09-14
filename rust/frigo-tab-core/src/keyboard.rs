/// Keys understood by the switcher boundary.  A platform adapter translates
/// native key codes into this deliberately small, portable set.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SwitcherKey {
    Unknown,
    Alt,
    Shift,
    Tab,
    Escape,
    F4,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    NumPad1,
    NumPad2,
    NumPad3,
    NumPad4,
    NumPad5,
    NumPad6,
    NumPad7,
    NumPad8,
    NumPad9,
}

/// Whether a keyboard event represents a press or a release.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeyTransition {
    Down,
    Up,
}

/// Identifies the physical modifier whose transition produced an event.
/// Keeping left and right keys distinct prevents releasing one key from
/// clearing the state of the other.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeyboardModifierKey {
    None,
    Alt,
    LeftAlt,
    RightAlt,
    Shift,
    LeftShift,
    RightShift,
}

/// A normalized keyboard event supplied by a platform adapter.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct KeyboardInput {
    pub key: SwitcherKey,
    pub transition: KeyTransition,
    pub alt: bool,
    pub shift: bool,
    pub injected: bool,
}

impl KeyboardInput {
    pub const fn new(
        key: SwitcherKey,
        transition: KeyTransition,
        alt: bool,
        shift: bool,
        injected: bool,
    ) -> Self {
        Self {
            key,
            transition,
            alt,
            shift,
            injected,
        }
    }

    pub const fn is_down(self) -> bool {
        matches!(self.transition, KeyTransition::Down)
    }

    pub const fn is_up(self) -> bool {
        matches!(self.transition, KeyTransition::Up)
    }
}
