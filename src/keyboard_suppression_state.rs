use std::sync::Mutex;

use crate::keyboard_input::{KeyboardInput, SwitcherKey};

/// Makes the bounded suppression decision required by LowLevelKeyboardProc.
/// Heavy session work is deliberately not part of this type.
pub struct KeyboardSuppressionState {
    state: Mutex<SuppressionState>,
}

#[derive(Default)]
struct SuppressionState {
    // SwitcherKey has fewer than 32 variants.  A mask keeps the hook path
    // allocation-free while retaining independent key-up bookkeeping.
    consumed_keys: u32,
    session_active_or_pending: bool,
    next_admission_token: i64,
    pending_admission_token: i64,
}

impl KeyboardSuppressionState {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(SuppressionState::default()),
        }
    }

    /// Returns the non-zero token only for the event that changes the
    /// switcher from idle to pending. Repeated Tab events deliberately receive
    /// no token, so a failed repeat delivery cannot cancel an accepted post.
    pub fn should_consume_with_token(&self, input: KeyboardInput) -> (bool, i64) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if input.injected {
            return (false, 0);
        }

        let key_bit = input.key.bit();
        if input.is_up() && state.consumed_keys & key_bit != 0 {
            state.consumed_keys &= !key_bit;
            return (true, 0);
        }
        if !input.is_down() {
            return (false, 0);
        }

        let consume = if !state.session_active_or_pending {
            let consume = input.key == SwitcherKey::Tab && input.alt;
            if consume {
                // Mark admission immediately, so repeat events that arrive
                // before the UI drains the first one stay balanced.
                state.session_active_or_pending = true;
                state.next_admission_token = next_token(state.next_admission_token);
                state.pending_admission_token = state.next_admission_token;
                state.consumed_keys |= key_bit;
                return (true, state.pending_admission_token);
            }
            false
        } else {
            (input.key == SwitcherKey::Tab && input.alt)
                || input.key == SwitcherKey::Escape
                || (input.key == SwitcherKey::F4 && input.alt)
                || is_digit(input.key)
        };

        if consume {
            state.consumed_keys |= key_bit;
        }
        (consume, 0)
    }

    pub fn set_session_visible(&self, visible: bool) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.session_active_or_pending = visible;
        state.pending_admission_token = 0;
    }

    /// Cancels only an admission that has not reached the UI.  The consumed
    /// ledger for a visible session is intentionally left untouched.
    pub fn abort_pending_admission(&self, admission_token: i64) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if admission_token == 0 || admission_token != state.pending_admission_token {
            return false;
        }
        state.pending_admission_token = 0;
        state.session_active_or_pending = false;
        state.consumed_keys &= !SwitcherKey::Tab.bit();
        true
    }

    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.session_active_or_pending = false;
        state.pending_admission_token = 0;
        state.consumed_keys = 0;
    }
}

impl Default for KeyboardSuppressionState {
    fn default() -> Self {
        Self::new()
    }
}

fn next_token(previous: i64) -> i64 {
    let next = previous.wrapping_add(1);
    if next == 0 { 1 } else { next }
}

fn is_digit(key: SwitcherKey) -> bool {
    matches!(
        key,
        SwitcherKey::D1
            | SwitcherKey::D2
            | SwitcherKey::D3
            | SwitcherKey::D4
            | SwitcherKey::D5
            | SwitcherKey::D6
            | SwitcherKey::D7
            | SwitcherKey::D8
            | SwitcherKey::D9
            | SwitcherKey::NumPad1
            | SwitcherKey::NumPad2
            | SwitcherKey::NumPad3
            | SwitcherKey::NumPad4
            | SwitcherKey::NumPad5
            | SwitcherKey::NumPad6
            | SwitcherKey::NumPad7
            | SwitcherKey::NumPad8
            | SwitcherKey::NumPad9
    )
}
