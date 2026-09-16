//! Native window primitives used by the switcher.
//!
//! The package keeps each public type in its own module while retaining the
//! original module names as compatibility surfaces.  In particular,
//! `window_handle` re-exports the style-bit wrappers and `window_icon`
//! re-exports the weak icon handle.

pub mod application_window;
pub mod application_windows;
pub mod frigo_window;
pub mod window_ex_styles;
pub mod window_finder;
pub mod window_handle;
pub mod window_icon;
pub mod window_icon_weak;
pub mod window_styles;

// These implementation-only modules keep the ownership/state helper types
// isolated as well; only the public window APIs are exposed by the package.
mod icon_info_bitmaps;
mod icon_state;
mod owned_icon;
mod window_type;

pub use application_window::ApplicationWindow;
pub use application_windows::ApplicationWindows;
pub use frigo_window::FrigoWindow;
pub use window_ex_styles::WindowExStyles;
pub use window_finder::WindowFinder;
pub use window_handle::WindowHandle;
pub use window_icon::{WindowIcon, WindowIconWeak};
pub use window_styles::WindowStyles;
