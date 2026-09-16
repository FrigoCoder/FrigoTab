//! Switcher domain package.
//!
//! The public modules retain the original crate paths through the root
//! compatibility aliases while the implementation types live in focused,
//! one-type-per-file modules.

mod alt_tab_behavior;
mod background_mode;
mod background_state;
mod session_painter;
mod snapshot_completion;
mod switcher_session_port;
mod window_dc;

pub mod session_window;
pub mod switcher_application;
pub mod switcher_state;

pub use session_window::{
    BackgroundMode, SessionPainter, SessionWindow, WM_DESKTOP_SNAPSHOT_READY,
};
pub use switcher_application::{AltTabBehavior, SwitcherApplication, SwitcherSessionPort};
pub use switcher_state::SwitcherState;
