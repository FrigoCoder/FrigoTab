use std::collections::HashSet;
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
    alt: HashSet<KeyboardModifierKey>,
    shift: HashSet<KeyboardModifierKey>,
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
            .expect("keyboard modifier state lock poisoned")
            .alt
            .len()
            > 0
    }

    pub fn shift_down(&self) -> bool {
        self.state
            .lock()
            .expect("keyboard modifier state lock poisoned")
            .shift
            .len()
            > 0
    }

    pub fn create_input(
        &self,
        key: SwitcherKey,
        transition: KeyTransition,
        native_alt_down: bool,
        injected: bool,
    ) -> KeyboardInput {
        self.create_input_with_modifier(
            key,
            transition,
            native_alt_down,
            injected,
            default_modifier(key),
        )
    }

    pub fn create_input_with_modifier(
        &self,
        key: SwitcherKey,
        transition: KeyTransition,
        native_alt_down: bool,
        injected: bool,
        physical_modifier: KeyboardModifierKey,
    ) -> KeyboardInput {
        let mut state = self
            .state
            .lock()
            .expect("keyboard modifier state lock poisoned");
        let is_down = matches!(transition, KeyTransition::Down);
        let alt =
            native_alt_down || !state.alt.is_empty() || (is_alt(physical_modifier) && is_down);
        let shift = !state.shift.is_empty() || (is_shift(physical_modifier) && is_down);

        let input = KeyboardInput::new(key, transition, alt, shift, injected);
        if injected {
            return input;
        }

        // Apply the transition after taking the snapshot.  Alt-up therefore
        // still describes the physical state before release.
        if is_alt(physical_modifier) {
            if is_down {
                state.alt.insert(physical_modifier);
            } else {
                state.alt.remove(&physical_modifier);
                // The generic Alt entry represents the LLKHF_ALTDOWN recovery
                // sentinel and is cleared by any physical Alt-up.
                state.alt.remove(&KeyboardModifierKey::Alt);
            }
        } else if is_shift(physical_modifier) {
            if is_down {
                state.shift.insert(physical_modifier);
            } else {
                state.shift.remove(&physical_modifier);
            }
        }
        if native_alt_down && state.alt.is_empty() && !is_alt(physical_modifier) {
            // LLKHF_ALTDOWN belongs to this event; remember it until Alt-up.
            state.alt.insert(KeyboardModifierKey::Alt);
        }

        input
    }

    pub fn reset(&self) {
        let mut state = self
            .state
            .lock()
            .expect("keyboard modifier state lock poisoned");
        state.alt.clear();
        state.shift.clear();
    }
}

impl Default for KeyboardModifierState {
    fn default() -> Self {
        Self::new()
    }
}

fn default_modifier(key: SwitcherKey) -> KeyboardModifierKey {
    match key {
        SwitcherKey::Alt => KeyboardModifierKey::Alt,
        SwitcherKey::Shift => KeyboardModifierKey::Shift,
        _ => KeyboardModifierKey::None,
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
