/// Keys understood by the switcher boundary.  The native adapter translates
/// virtual-key codes into this small, platform-independent set.
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

impl SwitcherKey {
    /// A compact identity used by the two consumed-key ledgers. `Unknown`
    /// deliberately has no bit because it is never suppressible.
    pub(crate) const fn bit(self) -> u32 {
        match self {
            Self::Unknown => 0,
            Self::Alt => 1 << 0,
            Self::Shift => 1 << 1,
            Self::Tab => 1 << 2,
            Self::Escape => 1 << 3,
            Self::F4 => 1 << 4,
            Self::D1 => 1 << 5,
            Self::D2 => 1 << 6,
            Self::D3 => 1 << 7,
            Self::D4 => 1 << 8,
            Self::D5 => 1 << 9,
            Self::D6 => 1 << 10,
            Self::D7 => 1 << 11,
            Self::D8 => 1 << 12,
            Self::D9 => 1 << 13,
            Self::NumPad1 => 1 << 14,
            Self::NumPad2 => 1 << 15,
            Self::NumPad3 => 1 << 16,
            Self::NumPad4 => 1 << 17,
            Self::NumPad5 => 1 << 18,
            Self::NumPad6 => 1 << 19,
            Self::NumPad7 => 1 << 20,
            Self::NumPad8 => 1 << 21,
            Self::NumPad9 => 1 << 22,
        }
    }
}

/// Whether a keyboard event represents a press or a release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

/// A normalized keyboard event supplied by the platform adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
