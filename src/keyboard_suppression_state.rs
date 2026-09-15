use crate::keyboard_input::{KeyboardInput, SwitcherKey};

/// Makes the bounded suppression decision required by LowLevelKeyboardProc.
/// Heavy session work is deliberately not part of this type.
#[derive(Default)]
pub struct KeyboardSuppressionState {
    // SwitcherKey has fewer than 32 variants.  A mask keeps the hook path
    // allocation-free while retaining independent key-up bookkeeping.
    consumed_keys: u32,
    session_active_or_pending: bool,
    next_admission_token: i64,
    pending_admission_token: i64,
}

impl KeyboardSuppressionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the non-zero token only for the event that changes the
    /// switcher from idle to pending. Repeated Tab events deliberately receive
    /// no token, so a failed repeat delivery cannot cancel an accepted post.
    pub fn should_consume_with_token(&mut self, input: KeyboardInput) -> (bool, i64) {
        if input.injected {
            return (false, 0);
        }

        let key_bit = input.key.bit();
        if input.is_up() && self.consumed_keys & key_bit != 0 {
            self.consumed_keys &= !key_bit;
            return (true, 0);
        }
        if !input.is_down() {
            return (false, 0);
        }

        let consume = if !self.session_active_or_pending {
            let consume = input.key == SwitcherKey::Tab && input.alt;
            if consume {
                // Mark admission immediately, so repeat events that arrive
                // before the UI drains the first one stay balanced.
                self.session_active_or_pending = true;
                self.next_admission_token = next_token(self.next_admission_token);
                self.pending_admission_token = self.next_admission_token;
                self.consumed_keys |= key_bit;
                return (true, self.pending_admission_token);
            }
            false
        } else {
            (input.key == SwitcherKey::Tab && input.alt)
                || input.key == SwitcherKey::Escape
                || (input.key == SwitcherKey::F4 && input.alt)
                || is_digit(input.key)
        };

        if consume {
            self.consumed_keys |= key_bit;
        }
        (consume, 0)
    }

    pub fn set_session_visible(&mut self, visible: bool) {
        self.session_active_or_pending = visible;
        self.pending_admission_token = 0;
    }

    /// Cancels only an admission that has not reached the UI.  The consumed
    /// ledger for a visible session is intentionally left untouched.
    pub fn abort_pending_admission(&mut self, admission_token: i64) -> bool {
        if admission_token == 0 || admission_token != self.pending_admission_token {
            return false;
        }
        self.pending_admission_token = 0;
        self.session_active_or_pending = false;
        self.consumed_keys &= !SwitcherKey::Tab.bit();
        true
    }

    pub fn reset(&mut self) {
        self.session_active_or_pending = false;
        self.pending_admission_token = 0;
        self.consumed_keys = 0;
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
