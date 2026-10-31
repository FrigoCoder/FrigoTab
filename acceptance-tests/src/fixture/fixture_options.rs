use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::WS_OVERLAPPEDWINDOW;

#[derive(Clone, Copy)]
pub struct FixtureOptions {
    pub bounds: RECT,
    pub style: u32,
    pub ex_style: u32,
    pub owner: HWND,
    pub activate: bool,
    pub initially_visible: bool,
    /// Model fullscreen renderers which discard their presentation surface
    /// when another application takes the foreground. The real HWND remains
    /// switchable, but becomes a black 1x1 window until it is reactivated.
    pub collapse_on_deactivate: bool,
}

impl Default for FixtureOptions {
    fn default() -> Self {
        Self {
            bounds: RECT {
                left: 100,
                top: 100,
                right: 500,
                bottom: 400,
            },
            style: WS_OVERLAPPEDWINDOW,
            ex_style: 0,
            owner: null_mut(),
            activate: false,
            initially_visible: true,
            collapse_on_deactivate: false,
        }
    }
}
