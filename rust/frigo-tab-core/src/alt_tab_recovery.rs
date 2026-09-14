use std::ops::Index;

use crate::{KeyTransition, KeyboardInput, SwitcherKey};

/// Maximum number of normalized events in a recovery gesture.
pub const MAX_RECOVERY_INPUTS: usize = 6;

/// A fixed-capacity synthetic Alt+Tab gesture.
///
/// Creating a plan never allocates.  Platform adapters can pass `as_slice()`
/// to their input injector or iterate over the plan by reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AltTabRecoveryPlan {
    inputs: [KeyboardInput; MAX_RECOVERY_INPUTS],
    len: u8,
}

impl AltTabRecoveryPlan {
    pub fn create(alt_still_down: bool, reverse: bool, shift_still_down: bool) -> Self {
        let mut plan = Self {
            inputs: [input(SwitcherKey::Unknown, KeyTransition::Up); MAX_RECOVERY_INPUTS],
            len: 0,
        };
        let synthetic_alt = !alt_still_down;
        let synthetic_shift = reverse && !shift_still_down;
        let temporarily_release_shift = !reverse && shift_still_down;

        if synthetic_alt {
            plan.push(input(SwitcherKey::Alt, KeyTransition::Down));
        }
        if synthetic_shift {
            plan.push(input(SwitcherKey::Shift, KeyTransition::Down));
        } else if temporarily_release_shift {
            plan.push(input(SwitcherKey::Shift, KeyTransition::Up));
        }

        plan.push(input(SwitcherKey::Tab, KeyTransition::Down));
        plan.push(input(SwitcherKey::Tab, KeyTransition::Up));

        if synthetic_shift {
            plan.push(input(SwitcherKey::Shift, KeyTransition::Up));
        } else if temporarily_release_shift {
            plan.push(input(SwitcherKey::Shift, KeyTransition::Down));
        }
        if synthetic_alt {
            plan.push(input(SwitcherKey::Alt, KeyTransition::Up));
        }
        plan
    }

    pub const fn len(&self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[KeyboardInput] {
        &self.inputs[..self.len as usize]
    }

    pub fn iter(&self) -> std::slice::Iter<'_, KeyboardInput> {
        self.as_slice().iter()
    }

    fn push(&mut self, input: KeyboardInput) {
        self.inputs[self.len as usize] = input;
        self.len += 1;
    }
}

impl Index<usize> for AltTabRecoveryPlan {
    type Output = KeyboardInput;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<'a> IntoIterator for &'a AltTabRecoveryPlan {
    type Item = &'a KeyboardInput;
    type IntoIter = std::slice::Iter<'a, KeyboardInput>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

fn input(key: SwitcherKey, transition: KeyTransition) -> KeyboardInput {
    KeyboardInput::new(key, transition, true, false, true)
}
