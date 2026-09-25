#![cfg(windows)]

//! Real Win32 drivers shared by the acceptance tests.
//!
//! This module contains real fixture HWNDs, a child process running the real
//! executable, bounded native polling, and the production objects needed by
//! the in-process scenarios. It has no substitute session or fake window API.

pub mod fixture;
pub mod live;
pub mod process;
pub mod screen;

pub use fixture::{FixtureOptions, FixtureWindow};
pub use live::{LiveSession, LiveTile};
pub use process::{RunningFrigoTab, TrayMenuItem, WM_BEGIN_SESSION};
pub use screen::{
    CursorPosition, ScreenCapture, capture_screen, capture_screen_image, enumerate_windows,
    find_pixel, find_screen_pixel, find_window_by_pid_and_bounds, foreground_window,
    get_window_rect, is_layered, is_window_visible, pixel_at, screen_pixel,
    visible_owned_layered_windows, visible_owned_overlays, window_bounds, window_ex_style,
    window_owner, window_style, window_title, windows_for_pid,
};

use std::ffi::OsStr;
use std::iter::once;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, PAINTSTRUCT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, DefWindowProcW, DispatchMessageW, GWLP_USERDATA,
    GetClientRect, GetWindowLongPtrW, IDC_ARROW, IDI_APPLICATION, LoadCursorW, LoadIconW, MSG,
    PM_REMOVE, PeekMessageW, RegisterClassExW, SetWindowLongPtrW, TranslateMessage, WM_ERASEBKGND,
    WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WNDCLASSEXW,
};

pub type Color = u32;
pub const RED: Color = rgb(255, 0, 0);
pub const GREEN: Color = rgb(0, 255, 0);
pub const BLUE: Color = rgb(0, 0, 255);
pub const MAGENTA: Color = rgb(255, 0, 255);
pub const ORANGE: Color = rgb(255, 165, 0);

pub const fn rgb(red: u8, green: u8, blue: u8) -> Color {
    red as Color | ((green as Color) << 8) | ((blue as Color) << 16)
}

pub fn serial_guard() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn set_per_monitor_dpi_awareness() -> bool {
    unsafe { !SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).is_null() }
}

pub fn use_per_monitor_physical_coordinates() -> bool {
    set_per_monitor_dpi_awareness()
}

pub const fn color_distance(first: Color, second: Color) -> u32 {
    (first & 0xff).abs_diff(second & 0xff)
        + ((first >> 8) & 0xff).abs_diff((second >> 8) & 0xff)
        + ((first >> 16) & 0xff).abs_diff((second >> 16) & 0xff)
}

pub fn pump_messages() {
    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

pub fn wait_until<F>(timeout: Duration, mut condition: F) -> bool
where
    F: FnMut() -> bool,
{
    let deadline = Instant::now() + timeout;
    loop {
        pump_messages();
        if condition() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn register_fixture_class() {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| unsafe {
        let class_name = fixture_class_name();
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(fixture_window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(null()),
            hIcon: LoadIconW(null_mut(), IDI_APPLICATION),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: LoadIconW(null_mut(), IDI_APPLICATION),
        };
        let _ = RegisterClassExW(&class);
    });
}
fn fixture_class_name() -> &'static Vec<u16> {
    static NAME: OnceLock<Vec<u16>> = OnceLock::new();
    NAME.get_or_init(|| wide("FrigoTab.AcceptanceFixture"))
}

unsafe extern "system" fn fixture_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    if message == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        if !create.is_null() {
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as isize) };
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }
    }
    match message {
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
            let mut client = RECT::default();
            unsafe { GetClientRect(hwnd, &mut client) };
            let brush = unsafe { CreateSolidBrush(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as u32) };
            if !brush.is_null() {
                unsafe {
                    FillRect(hdc, &client, brush);
                    DeleteObject(brush)
                };
            }
            unsafe { EndPaint(hwnd, &paint) };
            0
        }
        WM_ERASEBKGND => 1,
        WM_NCDESTROY => unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, message, wparam, lparam)
        },
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}
