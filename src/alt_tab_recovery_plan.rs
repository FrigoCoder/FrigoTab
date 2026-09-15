use crate::keyboard_input::{KeyTransition, KeyboardInput, SwitcherKey};

/// Describes the synthetic gesture needed when an initially suppressed Alt+Tab
/// cannot open the switcher.  The plan remains valid even when the user
/// released Alt before the UI message queue processed the request.
pub struct AltTabRecoveryPlan;

impl AltTabRecoveryPlan {
    pub fn create(
        alt_still_down: bool,
        reverse: bool,
        shift_still_down: bool,
    ) -> Vec<KeyboardInput> {
        let mut result = Vec::new();
        let synthetic_alt = !alt_still_down;
        let synthetic_shift = reverse && !shift_still_down;
        let temporarily_release_shift = !reverse && shift_still_down;

        if synthetic_alt {
            result.push(input(SwitcherKey::Alt, KeyTransition::Down));
        }
        if synthetic_shift {
            result.push(input(SwitcherKey::Shift, KeyTransition::Down));
        } else if temporarily_release_shift {
            result.push(input(SwitcherKey::Shift, KeyTransition::Up));
        }

        result.push(input(SwitcherKey::Tab, KeyTransition::Down));
        result.push(input(SwitcherKey::Tab, KeyTransition::Up));

        if synthetic_shift {
            result.push(input(SwitcherKey::Shift, KeyTransition::Up));
        } else if temporarily_release_shift {
            result.push(input(SwitcherKey::Shift, KeyTransition::Down));
        }
        if synthetic_alt {
            result.push(input(SwitcherKey::Alt, KeyTransition::Up));
        }
        result
    }
}

fn input(key: SwitcherKey, transition: KeyTransition) -> KeyboardInput {
    // Recovery input is marked injected so the low-level hook does not
    // suppress it again. It carries the same Alt=true snapshot as the
    // original adapter's synthetic gesture.
    KeyboardInput::new(key, transition, true, false, true)
}
