//! Platform-independent policy and geometry for FrigoTab.
//!
//! This crate deliberately contains no Windows (or other platform) bindings.
//! A platform adapter translates native keyboard events and owns the native
//! session surface through [`SwitcherSessionPort`].  The state machine here
//! is therefore straightforward to exercise with an in-memory port.

#![forbid(unsafe_code)]

mod alt_tab_recovery;
mod deferred_keyboard;
mod grid_layout;
mod keyboard;
mod keyboard_modifier;
mod keyboard_suppression;
mod layout;
mod screen;
mod switcher;

pub use alt_tab_recovery::{AltTabRecoveryPlan, MAX_RECOVERY_INPUTS};
pub use deferred_keyboard::{
    DeferredKeyboardDispatcher, DispatchTicket, DispatcherConfigError, DEFAULT_DISPATCH_CAPACITY,
    MAX_DISPATCH_SLOTS, MAX_ORDINARY_SLOTS,
};
pub use grid_layout::GridLayout;
pub use keyboard::{KeyTransition, KeyboardInput, KeyboardModifierKey, SwitcherKey};
pub use keyboard_modifier::KeyboardModifierState;
pub use keyboard_suppression::KeyboardSuppressionState;
pub use layout::{LayoutInputError, LayoutMonitor, LayoutWindow};
pub use screen::{ScreenPoint, ScreenRectangle};
pub use switcher::{
    KeyHandling, PortError, SwitcherApplication, SwitcherSessionPort, SwitcherState,
};
