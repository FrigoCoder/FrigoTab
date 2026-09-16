use std::ptr::{null, null_mut};

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

pub struct FixtureWindow {
    hwnd: HWND,
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
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                color as usize as *mut _,
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
        Self { hwnd }
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
