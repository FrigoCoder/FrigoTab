#![cfg(windows)]

//! FrigoTab's domain packages. Each package collects related Win32 and
//! switcher types; its modules keep one primary type per source file.

pub mod desktop;
pub mod geometry;
pub mod input;
pub mod rendering;
pub mod switcher;
pub mod system;
pub mod tray;
pub mod window;

// Preserve the original flat module paths for existing callers while new code
// can import from the packages above. These are aliases, not duplicate types.
pub use desktop::shell_desktop_snapshot;
pub use geometry::{layout, rect, screen_point};
pub use input::{
    alt_tab_recovery_plan, deferred_keyboard_dispatcher, key_handling, key_hook, keyboard_input,
    keyboard_modifier_state, keyboard_suppression_state,
};
pub use rendering as graphics;
pub use rendering::{gdi_plus, layer_updater, thumbnail};
pub use switcher::{session_window, switcher_application, switcher_state};
pub use system::single_instance_guard;
pub use tray::sys_tray_icon;
pub use window::{
    application_window, application_windows, frigo_window, window_finder, window_handle,
    window_icon,
};
