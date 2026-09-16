use crate::input::{KeyboardModifierState, KeyboardSuppressionState};

pub(crate) struct InputState {
    pub(crate) modifiers: KeyboardModifierState,
    pub(crate) suppression: KeyboardSuppressionState,
}
