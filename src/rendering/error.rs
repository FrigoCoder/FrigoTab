//! GDI+ status and error helpers.

use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Graphics::GdiPlus::Status;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTESESSION};

/// A GDI+ status code.  GDI+ does not use `GetLastError`; every operation
/// returns one of these values instead.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Error(pub Status);

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn check(status: Status) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(Error(status))
    }
}

// GDI+'s drawing operations deliberately tolerate the GDI+ generic
// and Win32 errors produced when the secure desktop or a remote session makes
// a display surface temporarily unavailable. This is its CheckErrorStatus
// behavior; setup/measurement operations continue to use strict `check`.
pub(crate) fn check_drawing(status: Status) -> Result<()> {
    if status == 0 {
        return Ok(());
    }
    if status == 1 || status == 7 {
        let error = unsafe { GetLastError() };
        if error == 5
            || error == 127
            || (error == 0 && unsafe { GetSystemMetrics(SM_REMOTESESSION) } & 1 != 0)
        {
            return Ok(());
        }
    }
    Err(Error(status))
}
