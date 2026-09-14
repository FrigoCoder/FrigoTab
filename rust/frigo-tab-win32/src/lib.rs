//! A deliberately small Win32 spike for FrigoTab.
//!
//! The public surface is intentionally narrower than `windows-sys`.  All raw
//! calls live in the private `windows` module and the owning types document the thread and
//! lifetime rules that Win32 expects.  This crate is a spike, not yet the
//! application shell.

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::*;

/// A compile-time inventory of the native surfaces covered by this spike.
///
/// This is intentionally a value-only diagnostic route.  Calling it never
/// creates a window, mutex, tray icon, GDI resource, DWM thumbnail, or global
/// keyboard hook, so it is safe to use from `cargo test` and build probes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmokeReport {
    /// Whether this crate was compiled for Windows.
    pub windows_target: bool,
    /// Whether the message-only/hidden-window surface is compiled.
    pub message_window: bool,
    /// Whether the single-instance and tray surface is compiled.
    pub single_instance_and_tray: bool,
    /// Whether the low-level keyboard-hook surface is compiled.
    pub keyboard_hook: bool,
    /// Whether the DWM thumbnail surface is compiled.
    pub dwm_thumbnail: bool,
    /// Whether the shell/GDI capture surface is compiled.
    pub shell_gdi_capture: bool,
}

impl SmokeReport {
    #[cfg(windows)]
    const WINDOWS: Self = Self {
        windows_target: true,
        message_window: true,
        single_instance_and_tray: true,
        keyboard_hook: true,
        dwm_thumbnail: true,
        shell_gdi_capture: true,
    };

    #[cfg(not(windows))]
    const NON_WINDOWS: Self = Self {
        windows_target: false,
        message_window: false,
        single_instance_and_tray: false,
        keyboard_hook: false,
        dwm_thumbnail: false,
        shell_gdi_capture: false,
    };
}

/// Return the no-side-effect native surface inventory.
pub fn smoke_report() -> SmokeReport {
    #[cfg(windows)]
    {
        SmokeReport::WINDOWS
    }

    #[cfg(not(windows))]
    {
        SmokeReport::NON_WINDOWS
    }
}

#[cfg(test)]
mod t20260914t181500z_smoke_contract;
