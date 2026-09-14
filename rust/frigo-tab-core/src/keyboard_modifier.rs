use crate::{KeyTransition, KeyboardInput, KeyboardModifierKey, SwitcherKey};

const ALT_SENTINEL: u8 = 1 << 0;
const ALT_LEFT: u8 = 1 << 1;
const ALT_RIGHT: u8 = 1 << 2;
const SHIFT_SENTINEL: u8 = 1 << 0;
const SHIFT_LEFT: u8 = 1 << 1;
const SHIFT_RIGHT: u8 = 1 << 2;

/// Derives modifier state from the low-level keyboard event stream.
///
/// The hook callback is a single-owner, latency-sensitive path.  Its state is
/// therefore represented by two fixed bitmasks rather than a lock or a hash
/// set.  The native adapter owns this value on its hook thread and calls the
/// mutating methods through `&mut self`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyboardModifierState {
    alt_keys: u8,
    shift_keys: u8,
}

impl KeyboardModifierState {
    pub const fn new() -> Self {
        Self {
            alt_keys: 0,
            shift_keys: 0,
        }
    }

    pub const fn alt_down(&self) -> bool {
        self.alt_keys != 0
    }

    pub const fn shift_down(&self) -> bool {
        self.shift_keys != 0
    }

    pub fn create_input(
        &mut self,
        key: SwitcherKey,
        transition: KeyTransition,
        reported_alt_down: bool,
        injected: bool,
    ) -> KeyboardInput {
        self.create_input_with_modifier(
            key,
            transition,
            reported_alt_down,
            injected,
            default_modifier(key),
        )
    }

    pub fn create_input_with_modifier(
        &mut self,
        key: SwitcherKey,
        transition: KeyTransition,
        reported_alt_down: bool,
        injected: bool,
        physical_modifier: KeyboardModifierKey,
    ) -> KeyboardInput {
        let is_down = matches!(transition, KeyTransition::Down);
        let alt = reported_alt_down || self.alt_down() || (is_alt(physical_modifier) && is_down);
        let shift = self.shift_down() || (is_shift(physical_modifier) && is_down);
        let input = KeyboardInput::new(key, transition, alt, shift, injected);

        if injected {
            return input;
        }

        // Take the snapshot before applying the transition.  In particular,
        // Alt-up still describes the state before the release.
        if let Some(bit) = alt_bit(physical_modifier) {
            if is_down {
                self.alt_keys |= bit;
            } else {
                self.alt_keys &= !bit;
                // The sentinel represents adapter-reported Alt context and
                // recovers a missed transition during listener setup/reset.
                self.alt_keys &= !ALT_SENTINEL;
            }
        } else if let Some(bit) = shift_bit(physical_modifier) {
            if is_down {
                self.shift_keys |= bit;
            } else {
                self.shift_keys &= !bit;
            }
        }
        if reported_alt_down && self.alt_keys == 0 && !is_alt(physical_modifier) {
            // The adapter metadata belongs to this event, rather than a
            // separately sampled asynchronous state. Remember it until Alt-up.
            self.alt_keys |= ALT_SENTINEL;
        }

        input
    }

    pub const fn reset(&mut self) {
        self.alt_keys = 0;
        self.shift_keys = 0;
    }
}

const fn default_modifier(key: SwitcherKey) -> KeyboardModifierKey {
    match key {
        SwitcherKey::Alt => KeyboardModifierKey::Alt,
        SwitcherKey::Shift => KeyboardModifierKey::Shift,
        _ => KeyboardModifierKey::None,
    }
}

const fn is_alt(modifier: KeyboardModifierKey) -> bool {
    matches!(
        modifier,
        KeyboardModifierKey::Alt | KeyboardModifierKey::LeftAlt | KeyboardModifierKey::RightAlt
    )
}

const fn is_shift(modifier: KeyboardModifierKey) -> bool {
    matches!(
        modifier,
        KeyboardModifierKey::Shift
            | KeyboardModifierKey::LeftShift
            | KeyboardModifierKey::RightShift
    )
}

const fn alt_bit(modifier: KeyboardModifierKey) -> Option<u8> {
    match modifier {
        KeyboardModifierKey::Alt => Some(ALT_SENTINEL),
        KeyboardModifierKey::LeftAlt => Some(ALT_LEFT),
        KeyboardModifierKey::RightAlt => Some(ALT_RIGHT),
        _ => None,
    }
}

const fn shift_bit(modifier: KeyboardModifierKey) -> Option<u8> {
    match modifier {
        KeyboardModifierKey::Shift => Some(SHIFT_SENTINEL),
        KeyboardModifierKey::LeftShift => Some(SHIFT_LEFT),
        KeyboardModifierKey::RightShift => Some(SHIFT_RIGHT),
        _ => None,
    }
}
