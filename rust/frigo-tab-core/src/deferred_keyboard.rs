use crate::KeyboardInput;

/// The ordinary queue is deliberately bounded.  One additional slot is kept
/// for a critical session-ending event, matching the native hook contract.
pub const MAX_ORDINARY_SLOTS: usize = 64;
pub const MAX_DISPATCH_SLOTS: usize = MAX_ORDINARY_SLOTS + 1;
pub const DEFAULT_DISPATCH_CAPACITY: usize = MAX_ORDINARY_SLOTS;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatcherConfigError {
    CapacityMustBePositive,
    CapacityExceedsFixedLimit,
}

impl std::fmt::Display for DispatcherConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapacityMustBePositive => {
                formatter.write_str("dispatcher capacity must be positive")
            }
            Self::CapacityExceedsFixedLimit => {
                write!(
                    formatter,
                    "dispatcher capacity exceeds {MAX_ORDINARY_SLOTS} slots"
                )
            }
        }
    }
}

impl std::error::Error for DispatcherConfigError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DispatchClass {
    Ordinary,
    Critical,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DispatchSlot {
    active: bool,
    sequence: u64,
    generation: u64,
    class: Option<DispatchClass>,
}

/// A copyable admission ticket.  The adapter owns delivery and calls
/// [`DeferredKeyboardDispatcher::complete`] or
/// [`DeferredKeyboardDispatcher::post_failed`] explicitly; this crate never
/// owns a callback, channel, or native UI object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchTicket {
    slot: u8,
    sequence: u64,
    generation: u64,
    class: DispatchClass,
    input: KeyboardInput,
}

impl DispatchTicket {
    pub const fn input(self) -> KeyboardInput {
        self.input
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn is_current(self, generation: u64) -> bool {
        self.generation == generation
    }
}

/// Allocation-free, platform-neutral queue admission state.
///
/// This models what the hook/UI bridge needs without trying to own the post
/// callback.  Admission is a fixed-array slot operation; stale events remain
/// accounted for until the platform reports completion or post failure, so a
/// delayed callback cannot release a newer event's slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeferredKeyboardDispatcher {
    slots: [DispatchSlot; MAX_DISPATCH_SLOTS],
    capacity: usize,
    ordinary_pending: usize,
    critical_pending: usize,
    generation: u64,
    next_sequence: u64,
}

impl DeferredKeyboardDispatcher {
    pub fn new(capacity: usize) -> Result<Self, DispatcherConfigError> {
        if capacity == 0 {
            return Err(DispatcherConfigError::CapacityMustBePositive);
        }
        if capacity > MAX_ORDINARY_SLOTS {
            return Err(DispatcherConfigError::CapacityExceedsFixedLimit);
        }
        Ok(Self {
            slots: [DispatchSlot::default(); MAX_DISPATCH_SLOTS],
            capacity,
            ordinary_pending: 0,
            critical_pending: 0,
            generation: 0,
            next_sequence: 0,
        })
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub const fn pending_count(&self) -> usize {
        self.ordinary_pending + self.critical_pending
    }

    pub const fn ordinary_pending(&self) -> usize {
        self.ordinary_pending
    }

    pub const fn critical_pending(&self) -> usize {
        self.critical_pending
    }

    /// Invalidate events which were admitted before an interruption.  Their
    /// slots remain bounded and must still be completed or marked failed.
    pub fn invalidate_pending(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn try_dispatch(&mut self, input: KeyboardInput) -> Option<DispatchTicket> {
        if self.ordinary_pending >= self.capacity {
            return None;
        }
        self.admit(input, DispatchClass::Ordinary)
    }

    pub fn try_dispatch_critical(&mut self, input: KeyboardInput) -> Option<DispatchTicket> {
        if self.critical_pending >= 1 {
            return None;
        }
        self.admit(input, DispatchClass::Critical)
    }

    /// Marks a posted callback as delivered.  A stale ticket is still
    /// released, but its `input()` must be discarded by the adapter when
    /// `is_current(ticket.generation())` is false.
    pub fn complete(&mut self, ticket: DispatchTicket) -> bool {
        self.release(ticket)
    }

    /// Returns whether a ticket still owns an active slot in the current
    /// generation.  Call this before [`Self::complete`]; completion then
    /// releases the exact slot even if the generation becomes stale later.
    pub fn should_deliver(&self, ticket: DispatchTicket) -> bool {
        let slot_index = ticket.slot as usize;
        if slot_index >= self.slots.len() || ticket.generation != self.generation {
            return false;
        }
        let slot = self.slots[slot_index];
        slot.active
            && slot.sequence == ticket.sequence
            && slot.generation == ticket.generation
            && slot.class == Some(ticket.class)
    }

    /// Marks a callback that could not be posted (or was dropped by the
    /// platform queue) as failed, releasing its exact slot.
    pub fn post_failed(&mut self, ticket: DispatchTicket) -> bool {
        self.release(ticket)
    }

    fn admit(&mut self, input: KeyboardInput, class: DispatchClass) -> Option<DispatchTicket> {
        let slot = self.slots.iter().position(|slot| !slot.active)?;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        if self.next_sequence == 0 {
            self.next_sequence = 1;
        }
        let sequence = self.next_sequence;
        self.slots[slot] = DispatchSlot {
            active: true,
            sequence,
            generation: self.generation,
            class: Some(class),
        };
        match class {
            DispatchClass::Ordinary => self.ordinary_pending += 1,
            DispatchClass::Critical => self.critical_pending += 1,
        }
        Some(DispatchTicket {
            slot: slot as u8,
            sequence,
            generation: self.generation,
            class,
            input,
        })
    }

    fn release(&mut self, ticket: DispatchTicket) -> bool {
        let slot_index = ticket.slot as usize;
        if slot_index >= self.slots.len() {
            return false;
        }
        let slot = &mut self.slots[slot_index];
        if !slot.active
            || slot.sequence != ticket.sequence
            || slot.generation != ticket.generation
            || slot.class != Some(ticket.class)
        {
            return false;
        }
        slot.active = false;
        match ticket.class {
            DispatchClass::Ordinary => self.ordinary_pending -= 1,
            DispatchClass::Critical => self.critical_pending -= 1,
        }
        true
    }
}

impl Default for DeferredKeyboardDispatcher {
    fn default() -> Self {
        Self::new(DEFAULT_DISPATCH_CAPACITY).expect("the fixed default capacity is valid")
    }
}
