use windows_sys::Win32::UI::WindowsAndMessaging::WM_APP;

/// Private owner-window query used by the external acceptance-test driver.
/// A nonzero result means the tray, session, and keyboard hook are installed.
pub const WM_APPLICATION_READY: u32 = WM_APP + 4;
