//! Switcher domain package.
//!
//! The public modules retain the original crate paths through the root
//! compatibility aliases while the implementation types live in focused,
//! one-type-per-file modules.

mod alt_tab_behavior;
mod background_mode;
mod background_state;
mod close_button_mode;
mod session_painter;
mod snapshot_completion;
mod switcher_session_port;
mod window_dc;

pub mod session_window;
pub mod switcher_application;
pub mod switcher_state;

pub use close_button_mode::CloseButtonMode;
pub use session_window::{
    APPLICATION_REFRESH_TIMER_ID, BackgroundMode, DISPLAY_RELAYOUT_TIMER_ID, SessionPainter,
    SessionWindow, WM_DESKTOP_SNAPSHOT_READY,
};
pub use switcher_application::{AltTabBehavior, SwitcherApplication, SwitcherSessionPort};
pub use switcher_state::SwitcherState;
