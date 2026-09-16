//! Native Explorer desktop capture and its GDI resource guards.

mod diagnostics;
mod gdi_memory_surface;
mod gdi_shell_desktop_snapshot_frame;
mod geometry;
mod shell_desktop_window_locator;
mod window_dc;

pub mod shell_desktop_snapshot;

pub use shell_desktop_snapshot::ShellDesktopSnapshot;
