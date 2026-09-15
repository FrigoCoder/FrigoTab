use crate::keyboard_input::{KeyTransition, KeyboardInput, SwitcherKey};

/// Describes the synthetic gesture needed when an initially suppressed Alt+Tab
/// cannot open the switcher.  The plan remains valid even when the user
/// released Alt before the UI message queue processed the request.
/// A bounded synthetic gesture stored entirely on the stack.
pub struct AltTabRecoveryPlan {
    inputs: [Option<KeyboardInput>; 6],
}

impl AltTabRecoveryPlan {
    pub fn create(alt_still_down: bool, reverse: bool, shift_still_down: bool) -> Self {
        let synthetic_alt = !alt_still_down;
        let synthetic_shift = reverse && !shift_still_down;
        let temporarily_release_shift = !reverse && shift_still_down;

        let shift_before = if synthetic_shift {
            Some(input(SwitcherKey::Shift, KeyTransition::Down))
        } else if temporarily_release_shift {
            Some(input(SwitcherKey::Shift, KeyTransition::Up))
        } else {
            None
        };
        let shift_after = if synthetic_shift {
            Some(input(SwitcherKey::Shift, KeyTransition::Up))
        } else if temporarily_release_shift {
            Some(input(SwitcherKey::Shift, KeyTransition::Down))
        } else {
            None
        };

        Self {
            inputs: [
                synthetic_alt.then(|| input(SwitcherKey::Alt, KeyTransition::Down)),
                shift_before,
                Some(input(SwitcherKey::Tab, KeyTransition::Down)),
                Some(input(SwitcherKey::Tab, KeyTransition::Up)),
                shift_after,
                synthetic_alt.then(|| input(SwitcherKey::Alt, KeyTransition::Up)),
            ],
        }
    }
}

impl IntoIterator for AltTabRecoveryPlan {
    type Item = KeyboardInput;
    type IntoIter = std::iter::Flatten<std::array::IntoIter<Option<KeyboardInput>, 6>>;

    fn into_iter(self) -> Self::IntoIter {
        self.inputs.into_iter().flatten()
    }
}

fn input(key: SwitcherKey, transition: KeyTransition) -> KeyboardInput {
    // Recovery input is marked injected so the low-level hook does not
    // suppress it again. It carries the same Alt=true snapshot as the
    // original adapter's synthetic gesture.
    KeyboardInput::new(key, transition, true, false, true)
}
