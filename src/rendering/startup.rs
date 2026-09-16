//! Process-wide GDI+ startup.

use std::ptr::null_mut;
use std::sync::OnceLock;

use windows_sys::Win32::Graphics::GdiPlus::{GdiplusStartup, GdiplusStartupInput, Status};

use super::error::{Error, Result};

static STARTUP_STATUS: OnceLock<Status> = OnceLock::new();

/// Start GDI+ once for the process.  It is intentionally never shut down:
/// all tile resources are released before process exit, and keeping the token
/// alive for the process lifetime matches the way GDI+ is used by
/// the original application.
pub fn ensure_started() -> Result<()> {
    let status = *STARTUP_STATUS.get_or_init(|| unsafe {
        let mut token = 0usize;
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            DebugEventCallback: 0,
            SuppressBackgroundThread: 0,
            SuppressExternalCodecs: 0,
        };
        GdiplusStartup(&mut token, &input, null_mut())
    });
    if status == 0 {
        Ok(())
    } else {
        Err(Error(status))
    }
}
