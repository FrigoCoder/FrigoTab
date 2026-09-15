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

use crate::rect::Rectangle;
use crate::screen_point::ScreenPoint;
use crate::window_handle::WindowHandle;

/// A screen-coordinate rectangle independent of any GUI toolkit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScreenRectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ScreenRectangle {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn right(self) -> i32 {
        self.x + self.width
    }

    pub const fn bottom(self) -> i32 {
        self.y + self.height
    }

    pub const fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    pub const fn contains(self, point: ScreenPoint) -> bool {
        !self.is_empty()
            && point.x >= self.x
            && point.x < self.right()
            && point.y >= self.y
            && point.y < self.bottom()
    }
}

impl std::hash::Hash for ScreenRectangle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // This is the original unchecked sequence, not Rust's tuple hash.
        let mut hash = self.x;
        hash = hash.wrapping_mul(397) ^ self.y;
        hash = hash.wrapping_mul(397) ^ self.width;
        hash = hash.wrapping_mul(397) ^ self.height;
        state.write_i32(hash);
    }
}

impl std::fmt::Display for ScreenRectangle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "({}, {}, {}, {})",
            self.x, self.y, self.width, self.height
        )
    }
}

/// Display bounds and usable working area for one monitor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutMonitor {
    pub id: String,
    pub bounds: ScreenRectangle,
    pub working_area: ScreenRectangle,
}

impl LayoutMonitor {
    pub fn new(
        id: impl Into<String>,
        bounds: ScreenRectangle,
        working_area: ScreenRectangle,
    ) -> Result<Self, LayoutInputError> {
        let id = id.into();
        if id.is_empty() {
            return Err(LayoutInputError::EmptyId);
        }
        Ok(Self {
            id,
            bounds,
            working_area,
        })
    }
}

/// A window candidate and the monitor to which its layout belongs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutWindow {
    pub id: String,
    pub monitor_id: String,
    pub bounds: ScreenRectangle,
}

impl LayoutWindow {
    pub fn new(
        id: impl Into<String>,
        monitor_id: impl Into<String>,
        bounds: ScreenRectangle,
    ) -> Result<Self, LayoutInputError> {
        let id = id.into();
        if id.is_empty() {
            return Err(LayoutInputError::EmptyId);
        }
        let monitor_id = monitor_id.into();
        if monitor_id.is_empty() {
            return Err(LayoutInputError::EmptyMonitorId);
        }
        Ok(Self {
            id,
            monitor_id,
            bounds,
        })
    }
}

/// Constructor failures corresponding to the original argument validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutInputError {
    EmptyId,
    EmptyMonitorId,
}

impl std::fmt::Display for LayoutInputError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => formatter.write_str("a layout identity needs a non-empty id"),
            Self::EmptyMonitorId => {
                formatter.write_str("a layout window needs a non-empty monitor id")
            }
        }
    }
}

impl std::error::Error for LayoutInputError {}

/// The core grid arrangement policy, copied from `GridLayout` without any
/// UI or window-manager dependencies.
#[derive(Clone, Copy, Debug, Default)]
pub struct GridLayout;

impl GridLayout {
    /// Arrange non-empty windows on their corresponding non-empty monitors.
    ///
    pub fn arrange(
        &self,
        windows: &[LayoutWindow],
        monitors: &[LayoutMonitor],
    ) -> HashMap<String, ScreenRectangle> {
        let mut windows_by_monitor: HashMap<&str, Vec<&LayoutWindow>> = HashMap::new();
        for window in windows {
            if window.bounds.is_empty() {
                continue;
            }
            windows_by_monitor
                .entry(window.monitor_id.as_str())
                .or_default()
                .push(window);
        }

        let mut result = HashMap::new();
        for monitor in monitors {
            if monitor.bounds.is_empty() || monitor.working_area.is_empty() {
                continue;
            }
            let Some(monitor_windows) = windows_by_monitor.get(monitor.id.as_str()) else {
                continue;
            };
            Self::layout_monitor_windows(monitor, monitor_windows, &mut result);
        }
        result
    }

    fn layout_monitor_windows(
        monitor: &LayoutMonitor,
        windows: &[&LayoutWindow],
        result: &mut HashMap<String, ScreenRectangle>,
    ) {
        // The original implementation performs these calculations in double for
        // the integer grid dimensions.
        let columns = (windows.len() as f64).sqrt().ceil() as usize;
        if columns == 0 {
            return;
        }
        let rows = ((windows.len() as f64) / columns as f64).ceil() as usize;
        if rows == 0 {
            return;
        }

        // The rest of the policy is deliberately f32, matching the original
        // single-precision behavior.
        let x_margin = monitor.bounds.width as f32 * 0.005_f32;
        let y_margin = monitor.bounds.height as f32 * 0.005_f32;
        let cell_width = monitor.working_area.width as f32 / columns as f32;
        let cell_height = monitor.working_area.height as f32 / rows as f32;

        for (index, window) in windows.iter().enumerate() {
            let column = index % columns;
            let row = index / columns;
            let cell_x = column as f32 * cell_width + x_margin;
            let cell_y = row as f32 * cell_height + y_margin;
            let available_width = cell_width - 2.0_f32 * x_margin;
            let available_height = cell_height - 2.0_f32 * y_margin;
            let bounds = Self::fit_inside(
                window.bounds,
                monitor.working_area.x as f32 + cell_x,
                monitor.working_area.y as f32 + cell_y,
                available_width,
                available_height,
            );
            result.insert(window.id.clone(), bounds);
        }
    }

    fn fit_inside(
        source: ScreenRectangle,
        cell_x: f32,
        cell_y: f32,
        cell_width: f32,
        cell_height: f32,
    ) -> ScreenRectangle {
        if source.is_empty() || cell_width <= 0.0 || cell_height <= 0.0 {
            return ScreenRectangle::new(0, 0, 0, 0);
        }

        let scale =
            1.0_f32.min((cell_width / source.width as f32).min(cell_height / source.height as f32));
        let width = source.width as f32 * scale;
        let height = source.height as f32 * scale;
        let x = cell_x + (cell_width - width) / 2.0_f32;
        let y = cell_y + (cell_height - height) / 2.0_f32;
        ScreenRectangle::new(
            round_to_even(x),
            round_to_even(y),
            round_to_even(width).max(1),
            round_to_even(height).max(1),
        )
    }
}

/// Round exactly as `Math.Round(double)`'s default ToEven mode.
///
/// The layout arithmetic is single precision, then promoted to double for the
/// same ties-to-even rounding used by the original implementation.
fn round_to_even(value: f32) -> i32 {
    (value as f64).round_ties_even() as i32
}

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

#[derive(Clone, Debug)]
struct MonitorNativeInfo {
    id: String,
    bounds: ScreenRectangle,
    working_area: ScreenRectangle,
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
    if unsafe {
        GetMonitorInfoW(
            monitor,
            (&mut info as *mut MONITORINFOEXW).cast::<MONITORINFO>(),
        )
    } == 0
    {
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
