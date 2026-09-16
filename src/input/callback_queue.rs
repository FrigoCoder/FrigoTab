use std::sync::Mutex;

pub(crate) type UiCallback = Box<dyn FnOnce() + Send + 'static>;

/// A fixed-size owner for callbacks posted to the UI queue.
///
/// Using numeric slots instead of putting a raw allocation in LPARAM means
/// the hook can release every callback if its owner HWND is destroyed before
/// Windows delivers the message.
pub(crate) struct CallbackQueue {
    slots: Mutex<Vec<Option<UiCallback>>>,
}

impl CallbackQueue {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            slots: Mutex::new((0..capacity + 1).map(|_| None).collect()),
        }
    }

    pub(crate) fn insert(&self, callback: UiCallback) -> Option<usize> {
        let mut slots = self.slots.lock().unwrap_or_else(|error| error.into_inner());
        let index = slots.iter().position(Option::is_none)?;
        slots[index] = Some(callback);
        Some(index + 1)
    }

    pub(crate) fn take(&self, slot: usize) -> Option<UiCallback> {
        if slot == 0 {
            return None;
        }
        let mut slots = self.slots.lock().unwrap_or_else(|error| error.into_inner());
        slots.get_mut(slot - 1)?.take()
    }

    pub(crate) fn drain(&self) {
        let mut slots = self.slots.lock().unwrap_or_else(|error| error.into_inner());
        for callback in slots.iter_mut() {
            // Dropping a callback releases its captured KeyboardInput and
            // any dispatcher release guard it owns.
            callback.take();
        }
    }
}
