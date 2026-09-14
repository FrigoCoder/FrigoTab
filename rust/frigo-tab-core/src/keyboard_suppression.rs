use crate::{KeyboardInput, SwitcherKey};

/// Makes the bounded suppression decision required by a global keyboard hook.
///
/// The hook thread is the sole owner of this value.  A bitset replaces the
/// previous lock and hash set: there is no blocking and no allocation in the
/// admission path, while an auto-repeat still needs only one matching release.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyboardSuppressionState {
    consumed_keys: u32,
    session_active_or_pending: bool,
    next_admission_token: u64,
    pending_admission_token: u64,
}

impl KeyboardSuppressionState {
    pub const fn new() -> Self {
        Self {
            consumed_keys: 0,
            session_active_or_pending: false,
            next_admission_token: 0,
            pending_admission_token: 0,
        }
    }

    pub const fn session_active_or_pending(&self) -> bool {
        self.session_active_or_pending
    }

    pub const fn admission_pending(&self) -> bool {
        self.pending_admission_token != 0
    }

    pub fn should_consume(&mut self, input: KeyboardInput) -> bool {
        self.should_consume_with_admission(input).0
    }

    /// Returns a non-zero token only for the event that changes the switcher
    /// from idle to pending.  Repeated Tab events deliberately get no token,
    /// so failure to post a repeat cannot cancel an already accepted session.
    pub fn should_consume_with_admission(&mut self, input: KeyboardInput) -> (bool, u64) {
        if input.injected {
            return (false, 0);
        }

        if input.is_up() {
            if let Some(bit) = key_bit(input.key) {
                if self.consumed_keys & bit != 0 {
                    self.consumed_keys &= !bit;
                    return (true, 0);
                }
            }
            return (false, 0);
        }
        if !input.is_down() {
            return (false, 0);
        }

        let mut admission_token = 0;
        let consume = if !self.session_active_or_pending {
            let consume = input.key == SwitcherKey::Tab && input.alt;
            if consume {
                self.session_active_or_pending = true;
                let token = self.next_admission_token();
                self.pending_admission_token = token;
                admission_token = token;
            }
            consume
        } else {
            (input.key == SwitcherKey::Tab && input.alt)
                || input.key == SwitcherKey::Escape
                || (input.key == SwitcherKey::F4 && input.alt)
                || is_digit(input.key)
        };

        if consume {
            if let Some(bit) = key_bit(input.key) {
                self.consumed_keys |= bit;
            }
        }
        (consume, admission_token)
    }

    pub const fn set_session_visible(&mut self, visible: bool) {
        self.session_active_or_pending = visible;
        self.pending_admission_token = 0;
    }

    /// Cancels only an admission which has not reached the UI.  This preserves
    /// the consumed-key ledger of an already visible session.
    pub fn abort_pending_admission(&mut self, admission_token: u64) -> bool {
        if admission_token == 0 || admission_token != self.pending_admission_token {
            return false;
        }
        self.pending_admission_token = 0;
        self.session_active_or_pending = false;
        if let Some(bit) = key_bit(SwitcherKey::Tab) {
            self.consumed_keys &= !bit;
        }
        true
    }

    pub const fn reset(&mut self) {
        self.session_active_or_pending = false;
        self.pending_admission_token = 0;
        self.consumed_keys = 0;
    }

    fn next_admission_token(&mut self) -> u64 {
        self.next_admission_token = self.next_admission_token.wrapping_add(1);
        if self.next_admission_token == 0 {
            self.next_admission_token = 1;
        }
        self.next_admission_token
    }
}

const fn key_bit(key: SwitcherKey) -> Option<u32> {
    let index = key as u32;
    if index < u32::BITS {
        Some(1u32 << index)
    } else {
        None
    }
}

const fn is_digit(key: SwitcherKey) -> bool {
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
