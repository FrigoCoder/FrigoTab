//! Error formatting helpers for layered surfaces.

use windows_sys::Win32::Foundation::GetLastError;

use super::gdi_plus;

pub(super) fn win32_error(operation: &str) -> String {
    format!("{operation} failed (Win32 error {})", unsafe {
        GetLastError()
    })
}

pub(super) fn gdiplus_error(operation: &str, error: gdi_plus::Error) -> String {
    format!("{operation} failed (GDI+ status {})", error.0)
}
