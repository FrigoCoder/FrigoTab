use windows_sys::Win32::Foundation::{HWND, RECT};

/// A source/popup pair observed from one in-process switcher session.
///
/// The source is the real fixture HWND selected by the production finder; the
/// popup is the real layered preview HWND created by `ApplicationWindow`.
#[derive(Clone, Copy)]
pub struct LiveTile {
    pub source: HWND,
    pub popup: HWND,
    pub bounds: RECT,
}
