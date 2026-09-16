pub use super::key_transition::KeyTransition;
pub use super::keyboard_modifier_key::KeyboardModifierKey;
pub use super::switcher_key::SwitcherKey;

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
