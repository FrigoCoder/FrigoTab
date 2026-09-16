//! Monitor discovery and the exact near-square arrangement used by FrigoTab.
//!
//! The first half of this module is the Rust counterpart of
//! shared layout module: its values are independent of the UI framework. The small
//! `Layout` wrapper at the end performs the Win32 monitor/window discovery
//! that the original `FrigoTab.Layout` performed through `System.Windows.Forms`.

use std::collections::HashMap;
use std::ffi::c_void;

use windows_sys::Win32::Foundation::{LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MONITORINFOEXW, MonitorFromRect,
};
use windows_sys::core::BOOL;

use super::monitor_native_info::MonitorNativeInfo;
use super::rect::Rectangle;
use crate::window::WindowHandle;

// Keep all layout value types available from `layout`, matching the original
// flat module API while each type remains in its own source file.
pub use super::grid_layout::GridLayout;
pub use super::layout_input_error::LayoutInputError;
pub use super::layout_monitor::LayoutMonitor;
pub use super::layout_window::LayoutWindow;
pub use super::screen_rectangle::ScreenRectangle;

/// The monitor-backed layout used by the native UI.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub bounds: HashMap<WindowHandle, Rectangle>,
}

impl Layout {
    pub fn new(windows: &[WindowHandle]) -> Self {
        let monitors = all_monitors();
        let mut ids: HashMap<String, WindowHandle> = HashMap::new();
        let mut candidates = Vec::new();

        for (index, &window) in windows.iter().enumerate() {
            let Some(restored) = window.try_get_rect() else {
                // HWNDs can disappear between EnumWindows and layout.  Omit
                // only this candidate and retain the rest of the session.
                continue;
            };
            let Some(monitor_id) = monitor_id_for_rectangle(restored) else {
                continue;
            };
            let id = index.to_string();
            ids.insert(id.clone(), window);
            // Native RECT has the same width/height semantics.
            candidates.push(
                LayoutWindow::new(
                    id,
                    monitor_id,
                    ScreenRectangle::new(0, 0, restored.width, restored.height),
                )
                .expect("non-empty generated layout identities"),
            );
        }

        let arranged = GridLayout.arrange(&candidates, &monitors);
        let mut bounds = HashMap::new();
        for (id, screen_rectangle) in arranged {
            if let Some(window) = ids.get(&id) {
                bounds.insert(
                    *window,
                    Rectangle::new(
                        screen_rectangle.x,
                        screen_rectangle.y,
                        screen_rectangle.width,
                        screen_rectangle.height,
                    ),
                );
            }
        }
        Self { bounds }
    }
}

fn all_monitors() -> Vec<LayoutMonitor> {
    let mut native: Vec<MonitorNativeInfo> = Vec::new();
    // SAFETY: The callback receives a pointer to this live Vec only during
    // this synchronous enumeration.
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(monitor_callback),
            &mut native as *mut _ as LPARAM,
        );
    }
    native
        .into_iter()
        .filter_map(|monitor| {
            LayoutMonitor::new(monitor.id, monitor.bounds, monitor.working_area).ok()
        })
        .collect()
}

unsafe extern "system" fn monitor_callback(
    monitor: HMONITOR,
    _dc: *mut c_void,
    _clip: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(lparam as *mut Vec<MonitorNativeInfo>) };
    if let Some(info) = monitor_info(monitor) {
        monitors.push(info);
    }
    1
}

fn monitor_info(monitor: HMONITOR) -> Option<MonitorNativeInfo> {
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    // GetMonitorInfoW receives a MONITORINFO pointer; MONITORINFOEXW starts
    // with that exact structure and carries the device name after it.
    if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) } == 0 {
        return None;
    }
    let id_end = info
        .szDevice
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(info.szDevice.len());
    let id = String::from_utf16_lossy(&info.szDevice[..id_end]);
    Some(MonitorNativeInfo {
        id,
        bounds: from_native_rect(info.monitorInfo.rcMonitor),
        working_area: from_native_rect(info.monitorInfo.rcWork),
    })
}

fn monitor_id_for_rectangle(rectangle: Rectangle) -> Option<String> {
    let native = RECT {
        left: rectangle.x,
        top: rectangle.y,
        right: rectangle.right(),
        bottom: rectangle.bottom(),
    };
    // Screen.FromRectangle returns the nearest display, which is the native
    // MONITOR_DEFAULTTONEAREST mode used here.
    let monitor = unsafe { MonitorFromRect(&native, MONITOR_DEFAULTTONEAREST) };
    (!monitor.is_null()).then(|| monitor_info(monitor).map(|info| info.id))?
}

const fn from_native_rect(rectangle: RECT) -> ScreenRectangle {
    ScreenRectangle::new(
        rectangle.left,
        rectangle.top,
        rectangle.right - rectangle.left,
        rectangle.bottom - rectangle.top,
    )
}
