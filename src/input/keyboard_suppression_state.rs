use crate::input::{KeyboardInput, SwitcherKey};

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
        if input.is_down() && !self.session_active_or_pending && self.consumed_keys & key_bit != 0 {
            // The session can close before key-repeat stops. Keep repeats
            // suppressed until the one physical up clears this ledger entry.
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
            // Once FrigoTab owns a gesture, every known physical key belongs
            // to that gesture until its matching release. This prevents keys
            // typed while the UI message is still pending from reaching the
            // old foreground application.
            input.key != SwitcherKey::Unknown
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

    /// Cancels only an admission that has not reached the UI. The physical
    /// key-down was already suppressed, so its consumed-key entry must remain
    /// until the matching key-up even when native Alt+Tab recovery is needed.
    pub fn abort_pending_admission(&mut self, admission_token: i64) -> bool {
        if admission_token == 0 || admission_token != self.pending_admission_token {
            return false;
        }
        self.pending_admission_token = 0;
        self.session_active_or_pending = false;
        true
    }

    pub(crate) fn session_active_or_pending(&self) -> bool {
        self.session_active_or_pending
    }

    pub(crate) fn consume_matching_release(&mut self, key: SwitcherKey) {
        self.consumed_keys |= key.bit();
    }

    pub(crate) fn consumes_matching_release(&self, key: SwitcherKey) -> bool {
        self.consumed_keys & key.bit() != 0
    }

    pub(crate) fn forget_matching_release(&mut self, key: SwitcherKey) {
        self.consumed_keys &= !key.bit();
    }

    /// Reset the active admission without exposing releases for key-downs the
    /// hook already suppressed. Full shutdown uses `reset` instead.
    pub(crate) fn reset_session_state(&mut self) {
        self.session_active_or_pending = false;
        self.pending_admission_token = 0;
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
