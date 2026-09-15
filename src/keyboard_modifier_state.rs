use std::sync::Mutex;

use crate::keyboard_input::{KeyTransition, KeyboardInput, KeyboardModifierKey, SwitcherKey};

/// Derives modifier state from the low-level keyboard event stream.
///
/// Low-level hooks run before Windows updates asynchronous keyboard state, so
/// sampling GetAsyncKeyState from inside the callback is stale.  The hook
/// records modifier transitions here and asks this type for the input snapshot
/// that belongs to each event.
pub struct KeyboardModifierState {
    state: Mutex<ModifierKeys>,
}

#[derive(Default)]
struct ModifierKeys {
    // There are only three physical variants for each modifier.  Keeping
    // these as masks avoids a HashSet allocation in the low-level hook.
    alt: u8,
    shift: u8,
}

impl KeyboardModifierState {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(ModifierKeys::default()),
        }
    }

    pub fn alt_down(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .alt
            != 0
    }

    pub fn shift_down(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .shift
            != 0
    }

    pub fn create_input_with_modifier(
        &self,
        key: SwitcherKey,
        transition: KeyTransition,
        native_alt_down: bool,
        injected: bool,
        physical_modifier: KeyboardModifierKey,
    ) -> KeyboardInput {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let is_down = matches!(transition, KeyTransition::Down);
        let alt = native_alt_down || state.alt != 0 || (is_alt(physical_modifier) && is_down);
        let shift = state.shift != 0 || (is_shift(physical_modifier) && is_down);

        let input = KeyboardInput::new(key, transition, alt, shift, injected);
        if injected {
            return input;
        }

        // Apply the transition after taking the snapshot.  Alt-up therefore
        // still describes the physical state before release.
        if is_alt(physical_modifier) {
            if is_down {
                state.alt |= modifier_bit(physical_modifier);
            } else {
                state.alt &= !modifier_bit(physical_modifier);
                // The generic Alt entry represents the LLKHF_ALTDOWN recovery
                // sentinel and is cleared by any physical Alt-up.
                state.alt &= !modifier_bit(KeyboardModifierKey::Alt);
            }
        } else if is_shift(physical_modifier) {
            if is_down {
                state.shift |= modifier_bit(physical_modifier);
            } else {
                state.shift &= !modifier_bit(physical_modifier);
            }
        }
        if native_alt_down && state.alt == 0 && !is_alt(physical_modifier) {
            // LLKHF_ALTDOWN belongs to this event; remember it until Alt-up.
            state.alt |= modifier_bit(KeyboardModifierKey::Alt);
        }

        input
    }

    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.alt = 0;
        state.shift = 0;
    }
}

impl Default for KeyboardModifierState {
    fn default() -> Self {
        Self::new()
    }
}

fn is_alt(modifier: KeyboardModifierKey) -> bool {
    matches!(
        modifier,
        KeyboardModifierKey::Alt | KeyboardModifierKey::LeftAlt | KeyboardModifierKey::RightAlt
    )
}

fn is_shift(modifier: KeyboardModifierKey) -> bool {
    matches!(
        modifier,
        KeyboardModifierKey::Shift
            | KeyboardModifierKey::LeftShift
            | KeyboardModifierKey::RightShift
    )
}

fn modifier_bit(modifier: KeyboardModifierKey) -> u8 {
    match modifier {
        KeyboardModifierKey::Alt => 1 << 0,
        KeyboardModifierKey::LeftAlt => 1 << 1,
        KeyboardModifierKey::RightAlt => 1 << 2,
        KeyboardModifierKey::Shift => 1 << 0,
        KeyboardModifierKey::LeftShift => 1 << 1,
        KeyboardModifierKey::RightShift => 1 << 2,
        KeyboardModifierKey::None => 0,
    }
}
