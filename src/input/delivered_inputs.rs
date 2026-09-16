use std::collections::VecDeque;
use std::sync::Mutex;

use crate::input::KeyboardInput;

pub(crate) struct DeliveredInputs {
    inputs: Mutex<VecDeque<KeyboardInput>>,
}

impl DeliveredInputs {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            inputs: Mutex::new(VecDeque::with_capacity(capacity + 1)),
        }
    }

    pub(crate) fn push(&self, input: KeyboardInput, capacity: usize) {
        let mut inputs = self
            .inputs
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        // One UI message consumes one callback before the next is dispatched.
        // Keep this defensive bound in case a future dispatcher changes its
        // ordering; dropping here is safer than allocating in the UI bridge.
        if inputs.len() < capacity + 1 {
            inputs.push_back(input);
        }
    }

    pub(crate) fn pop(&self) -> Option<KeyboardInput> {
        self.inputs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .pop_front()
    }
}
