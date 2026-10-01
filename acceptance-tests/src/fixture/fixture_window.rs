use std::collections::VecDeque;
use std::ptr::{null, null_mut};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::UpdateWindow;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, IsWindow, SW_HIDE, SW_MINIMIZE, SW_RESTORE, SW_SHOWNA,
    SWP_NOACTIVATE, SetForegroundWindow, SetWindowPos, SetWindowTextW, ShowWindow,
};

use super::fixture_options::FixtureOptions;
use crate::{
    Color, fixture_class_name, pump_messages, register_fixture_class, wide, window_bounds,
    window_title,
};

pub(crate) struct FixtureWindowState {
    pub(crate) color: Color,
    keyboard_messages: Mutex<VecDeque<KeyboardMessage>>,
    keyboard_message_log_overflowed: AtomicBool,
    system_close_requests: AtomicUsize,
}

/// A keyboard message delivered to a real fixture HWND.
///
/// The fields mirror the documented bit layout of a keyboard message's
/// `lParam`.  Keeping the raw Win32 message alongside the decoded flags lets
/// acceptance tests distinguish replayed `WM_SYSKEY*` messages from ordinary
/// key messages and verify exact down/up ordering without inspecting private
/// application state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyboardMessage {
    pub message: u32,
    pub virtual_key: u16,
    pub foreground_window: usize,
    pub repeat_count: u16,
    pub scan_code: u8,
    pub extended: bool,
    pub alt_context: bool,
    pub previous_state: bool,
    pub transition: bool,
}

const MAX_KEYBOARD_MESSAGES: usize = 512;

impl FixtureWindowState {
    fn new(color: Color) -> Self {
        Self {
            color,
            keyboard_messages: Mutex::new(VecDeque::with_capacity(MAX_KEYBOARD_MESSAGES)),
            keyboard_message_log_overflowed: AtomicBool::new(false),
            system_close_requests: AtomicUsize::new(0),
        }
    }

    fn clear_keyboard_messages(&self) {
        self.keyboard_messages
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clear();
        self.keyboard_message_log_overflowed
            .store(false, Ordering::Release);
    }

    pub(crate) fn record_keyboard_message(
        &self,
        message: u32,
        virtual_key: usize,
        lparam: isize,
        foreground_window: usize,
    ) {
        let bits = lparam as u32;
        let mut messages = self
            .keyboard_messages
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if messages.len() >= MAX_KEYBOARD_MESSAGES {
            self.keyboard_message_log_overflowed
                .store(true, Ordering::Release);
            return;
        }
        messages.push_back(KeyboardMessage {
            message,
            virtual_key: virtual_key as u16,
            foreground_window,
            repeat_count: (bits & 0xffff) as u16,
            scan_code: ((bits >> 16) & 0xff) as u8,
            extended: bits & (1 << 24) != 0,
            alt_context: bits & (1 << 29) != 0,
            previous_state: bits & (1 << 30) != 0,
            transition: bits & (1 << 31) != 0,
        });
    }

    fn keyboard_message_count(&self) -> usize {
        self.keyboard_messages
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len()
    }

    fn keyboard_messages(&self) -> Vec<KeyboardMessage> {
        self.keyboard_messages
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .copied()
            .collect()
    }

    fn keyboard_message_log_overflowed(&self) -> bool {
        self.keyboard_message_log_overflowed.load(Ordering::Acquire)
    }

    pub(crate) fn record_system_close_request(&self) {
        self.system_close_requests.fetch_add(1, Ordering::AcqRel);
    }

    fn system_close_request_count(&self) -> usize {
        self.system_close_requests.load(Ordering::Acquire)
    }
}

pub struct FixtureWindow {
    hwnd: HWND,
    state: Box<FixtureWindowState>,
}

impl FixtureWindow {
    pub fn show(title: &str, color: Color, bounds: RECT) -> Self {
        Self::show_with_options(
            title,
            color,
            FixtureOptions {
                bounds,
                ..Default::default()
            },
        )
    }

    pub fn show_with_options(title: &str, color: Color, options: FixtureOptions) -> Self {
        register_fixture_class();
        let title = wide(title);
        let state = Box::new(FixtureWindowState::new(color));
        let state_pointer = state.as_ref() as *const FixtureWindowState as *mut _;
        let hwnd = unsafe {
            CreateWindowExW(
                options.ex_style,
                fixture_class_name().as_ptr(),
                title.as_ptr(),
                options.style,
                options.bounds.left,
                options.bounds.top,
                options.bounds.right.saturating_sub(options.bounds.left),
                options.bounds.bottom.saturating_sub(options.bounds.top),
                options.owner,
                null_mut(),
                GetModuleHandleW(null()),
                state_pointer,
            )
        };
        assert!(!hwnd.is_null(), "CreateWindowExW failed for fixture window");
        if options.initially_visible {
            unsafe {
                ShowWindow(
                    hwnd,
                    if options.activate {
                        SW_RESTORE
                    } else {
                        SW_SHOWNA
                    },
                );
                if options.activate {
                    SetForegroundWindow(hwnd);
                }
                UpdateWindow(hwnd);
            };
        }
        pump_messages();
        Self { hwnd, state }
    }

    pub fn handle(&self) -> HWND {
        self.hwnd
    }

    pub fn bounds(&self) -> RECT {
        window_bounds(self.hwnd).unwrap_or_default()
    }

    pub fn set_bounds(&self, bounds: RECT) -> bool {
        if self.hwnd.is_null() {
            return false;
        }
        let ok = unsafe {
            SetWindowPos(
                self.hwnd,
                null_mut(),
                bounds.left,
                bounds.top,
                bounds.right.saturating_sub(bounds.left),
                bounds.bottom.saturating_sub(bounds.top),
                SWP_NOACTIVATE,
            ) != 0
        };
        pump_messages();
        ok
    }

    pub fn title(&self) -> String {
        window_title(self.hwnd)
    }

    pub fn clear_keyboard_messages(&self) {
        pump_messages();
        self.state.clear_keyboard_messages();
    }

    pub fn keyboard_message_count(&self) -> usize {
        pump_messages();
        self.state.keyboard_message_count()
    }

    /// Returns the exact bounded sequence delivered to this fixture HWND.
    /// Call [`Self::keyboard_message_log_overflowed`] before relying on the
    /// sequence for an exhaustive assertion.
    pub fn keyboard_messages(&self) -> Vec<KeyboardMessage> {
        pump_messages();
        self.state.keyboard_messages()
    }

    /// Reports whether more messages arrived than the bounded acceptance log
    /// can retain.  Tests should fail or clear the log before making an exact
    /// sequence assertion when this is true.
    pub fn keyboard_message_log_overflowed(&self) -> bool {
        pump_messages();
        self.state.keyboard_message_log_overflowed()
    }

    /// Counts title-bar/system-menu close commands delivered to this real
    /// fixture HWND. This distinguishes native `SC_CLOSE` behavior from a
    /// synthetic `WM_CLOSE` message while keeping normal DefWindowProc
    /// destruction intact.
    pub fn system_close_request_count(&self) -> usize {
        pump_messages();
        self.state.system_close_request_count()
    }

    pub fn set_title(&self, title: &str) -> bool {
        let title = wide(title);
        unsafe { SetWindowTextW(self.hwnd, title.as_ptr()) != 0 }
    }

    pub fn hide(&self) {
        unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        pump_messages();
    }

    pub fn show_again(&self) {
        unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        pump_messages();
    }

    pub fn minimize(&self) {
        unsafe { ShowWindow(self.hwnd, SW_MINIMIZE) };
        pump_messages();
    }

    pub fn restore(&self) {
        unsafe { ShowWindow(self.hwnd, SW_RESTORE) };
        pump_messages();
    }

    pub fn close(&mut self) {
        self.destroy();
    }

    fn destroy(&mut self) {
        if !self.hwnd.is_null() && unsafe { IsWindow(self.hwnd) } != 0 {
            unsafe { DestroyWindow(self.hwnd) };
            pump_messages();
        }
        self.hwnd = null_mut();
    }
}

impl Drop for FixtureWindow {
    fn drop(&mut self) {
        self.destroy();
    }
}
