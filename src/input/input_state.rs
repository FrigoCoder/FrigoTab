use crate::input::{KeyboardModifierState, KeyboardSuppressionState};

pub(crate) struct InputState {
    pub(crate) modifiers: KeyboardModifierState,
    pub(crate) suppression: KeyboardSuppressionState,
    // A physical Alt-down is held until the next key identifies whether this
    // is FrigoTab's gesture or an ordinary Alt shortcut. Keeping the original
    // native details preserves left/right Alt and AltGr when replay is needed.
    pub(crate) deferred_alt_vk: u16,
    pub(crate) deferred_alt_scan: u16,
    pub(crate) deferred_alt_extended: bool,
    pub(crate) deferred_shift_vk: u16,
    pub(crate) deferred_shift_scan: u16,
    pub(crate) deferred_shift_extended: bool,
    // Once an ordinary Alt shortcut has been replayed, the remaining physical
    // chord belongs to Windows until every Alt key is released.
    pub(crate) alt_forwarded: bool,
    // Failed native replay must not expose unmatched physical key-up events.
    // Virtual-key codes are bytes, so four words form a bounded, allocation-
    // free release ledger for the hook callback.
    pub(crate) blocked_native_releases: [u64; 4],
}
