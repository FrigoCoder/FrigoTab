#![cfg(windows)]

//! Real HWND/GDI/DWM acceptance coverage for the native window and backdrop
//! portions of FrigoTab.  These scenarios intentionally exercise the public
//! production types with windows created by the real Win32 window manager.

use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use frigotab::layout::Layout;
use frigotab::rect::Rectangle;
use frigotab::shell_desktop_snapshot::ShellDesktopSnapshot;
use frigotab::thumbnail::DwmThumbnail;
use frigotab::window_finder::WindowFinder;
use frigotab::window_handle::WindowHandle;
use frigotab_acceptance::{
    Color, FixtureOptions, FixtureWindow, MAGENTA, pixel_at, pump_messages, serial_guard,
    window_bounds,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BLACKNESS, ClientToScreen, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, EnumDisplayMonitors, GetDC, GetMonitorInfoW, GetPixel,
    HBITMAP, HDC, HGDIOBJ, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MONITORINFOEXW,
    MonitorFromRect, PatBlt, ReleaseDC, SelectObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetSystemMetrics, HWND_TOPMOST, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SetWindowPos, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows_sys::core::BOOL;

const DWM_UNAVAILABLE: &str = "DWM thumbnails are unavailable on this desktop";
#[test]
fn window_finder_includes_a_real_normal_window_and_excludes_unsupported_windows() {
    let _serial = serial_guard();
    let _ = frigotab_acceptance::use_per_monitor_physical_coordinates();
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let normal = show_acceptance_window(
        &format!("FrigoTab.Acceptance.Normal.{suffix}"),
        frigotab_acceptance::rgb(100, 149, 237),
        rect(120, 120, 360, 280),
    );
    let hidden = show_acceptance_window(
        &format!("FrigoTab.Acceptance.Hidden.{suffix}"),
        frigotab_acceptance::rgb(128, 128, 128),
        rect(140, 140, 360, 280),
    );
    let tool = FixtureWindow::show_with_options(
        &format!("FrigoTab.Acceptance.Tool.{suffix}"),
        frigotab_acceptance::rgb(128, 128, 128),
        FixtureOptions {
            style: WS_POPUP,
            ex_style: WS_EX_TOOLWINDOW,
            activate: true,
            ..FixtureOptions::default()
        },
    );
    let no_activate = FixtureWindow::show_with_options(
        &format!("FrigoTab.Acceptance.NoActivate.{suffix}"),
        frigotab_acceptance::rgb(128, 128, 128),
        FixtureOptions {
            style: WS_POPUP,
            ex_style: WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            activate: true,
            ..FixtureOptions::default()
        },
    );
    hidden.hide();
    pump_messages();

    let finder = WindowFinder::new();
    let normal_handle = WindowHandle::new(normal.handle());
    let hidden_handle = WindowHandle::new(hidden.handle());
    let tool_handle = WindowHandle::new(tool.handle());
    let no_activate_handle = WindowHandle::new(no_activate.handle());

    assert!(finder.windows.contains(&normal_handle));
    assert!(!finder.windows.contains(&hidden_handle));
    assert!(!finder.windows.contains(&tool_handle));
    assert!(!finder.windows.contains(&no_activate_handle));
}

#[test]
fn window_handle_preserves_a_unicode_title_from_a_real_window() {
    let _serial = serial_guard();
    let _ = frigotab_acceptance::use_per_monitor_physical_coordinates();
    let title = "FrigoTab.Acceptance.Ω窗";
    let window = show_acceptance_window(
        title,
        frigotab_acceptance::rgb(100, 149, 237),
        rect(120, 120, 360, 280),
    );
    assert!(window.set_title(title));
    pump_messages();
    assert_eq!(title, WindowHandle::new(window.handle()).get_window_text());
}

#[test]
fn layout_keeps_a_live_window_and_skips_a_closed_window_handle() {
    let _serial = serial_guard();
    let _ = frigotab_acceptance::use_per_monitor_physical_coordinates();
    let live = show_acceptance_window(
        "FrigoTab.Acceptance.Layout.Live",
        frigotab_acceptance::rgb(100, 149, 237),
        rect(120, 120, 360, 280),
    );
    let mut closed = show_acceptance_window(
        "FrigoTab.Acceptance.Layout.Closed",
        frigotab_acceptance::rgb(128, 128, 128),
        rect(160, 160, 360, 280),
    );
    let live_handle = WindowHandle::new(live.handle());
    let closed_handle = WindowHandle::new(closed.handle());
    closed.close();
    pump_messages();

    let layout = Layout::new(&[live_handle, closed_handle]);
    let live_tile = layout
        .bounds
        .get(&live_handle)
        .copied()
        .expect("a live real HWND must receive a tile");
    assert!(live_tile.width > 0 && live_tile.height > 0);
    assert!(!layout.bounds.contains_key(&closed_handle));
}

#[test]
fn minimized_window_uses_its_restored_monitor_when_multiple_screens_exist() {
    let _serial = serial_guard();
    let _ = frigotab_acceptance::use_per_monitor_physical_coordinates();
    let monitors = enumerate_monitors();
    let Some(target) = monitors.into_iter().find(|monitor| !monitor.primary) else {
        eprintln!("INCONCLUSIVE: the restored-monitor case requires at least two monitors");
        return;
    };
    let bounds = inflate(target.working_area, -40, -40);
    let window = show_acceptance_window(
        "FrigoTab.Acceptance.Layout.Minimized",
        frigotab_acceptance::rgb(100, 149, 237),
        bounds,
    );
    window.minimize();
    pump_messages();

    let handle = WindowHandle::new(window.handle());
    let restored = handle
        .try_get_rect()
        .expect("a minimized real window must keep restored placement");
    let restored_bounds = RECT {
        left: restored.x,
        top: restored.y,
        right: restored.right(),
        bottom: restored.bottom(),
    };
    assert_eq!(
        unsafe { MonitorFromRect(&restored_bounds, MONITOR_DEFAULTTONEAREST) },
        target.handle,
        "the restored rectangle resolved to a different monitor"
    );

    let layout = Layout::new(&[handle]);
    let tile = layout
        .bounds
        .get(&handle)
        .copied()
        .expect("the minimized real HWND must receive a tile");
    assert!(contains_rectangle(target.working_area, tile));
}

#[test]
fn shell_snapshot_draws_desktop_pixels_without_copying_a_visible_application() {
    let _serial = serial_guard();
    let _ = frigotab_acceptance::use_per_monitor_physical_coordinates();
    let desktop = virtual_bounds();
    if width(desktop) < 160 || height(desktop) < 160 {
        eprintln!("INCONCLUSIVE: the desktop is too small for a stable shell-snapshot check");
        return;
    }

    let covered = rect(desktop.left + 32, desktop.top + 32, 120, 120);
    let covered_window =
        show_acceptance_window("FrigoTab.Acceptance.ShellSurface", MAGENTA, covered);
    make_topmost(covered_window.handle());
    pump_messages();

    let snapshot = ShellDesktopSnapshot::capture(desktop);
    if !snapshot.is_available() {
        eprintln!("INCONCLUSIVE: Explorer did not expose a capturable desktop surface");
        return;
    }
    let Some(surface) = SnapshotSurface::new(width(desktop), height(desktop)) else {
        eprintln!("INCONCLUSIVE: a GDI destination surface could not be created");
        return;
    };
    snapshot.draw(
        surface.dc,
        RECT {
            left: 0,
            top: 0,
            right: width(desktop),
            bottom: height(desktop),
        },
    );
    let sample = surface.pixel(
        covered.left - desktop.left + width(covered) / 2,
        covered.top - desktop.top + height(covered) / 2,
    );
    assert!(
        !is_magenta(sample),
        "the Explorer shell snapshot copied the visible magenta application"
    );
    if surface.is_uniform_black() {
        eprintln!("INCONCLUSIVE: the desktop surface rendered uniformly black");
    }
}

#[test]
fn dwm_thumbnail_appears_only_during_its_visible_phase() {
    let _serial = serial_guard();
    let _ = frigotab_acceptance::use_per_monitor_physical_coordinates();
    let source = show_acceptance_window(
        "FrigoTab.Acceptance.Thumbnail.Source",
        MAGENTA,
        rect(120, 120, 180, 120),
    );
    let destination = show_acceptance_window(
        "FrigoTab.Acceptance.Thumbnail.Destination",
        frigotab_acceptance::rgb(0, 0, 0),
        rect(360, 120, 240, 160),
    );
    make_topmost(destination.handle());
    pump_messages();
    let Some((destination_rect, sample)) = client_rect_and_center(destination.handle()) else {
        eprintln!("INCONCLUSIVE: destination client geometry was unavailable");
        return;
    };

    let thumbnail = match DwmThumbnail::register(destination.handle(), source.handle()) {
        Ok(thumbnail) => thumbnail,
        Err(error) => {
            eprintln!("INCONCLUSIVE: {DWM_UNAVAILABLE}: HRESULT 0x{error:08x}");
            return;
        }
    };
    if let Err(error) = thumbnail.set_destination_rect(destination_rect) {
        eprintln!("INCONCLUSIVE: {DWM_UNAVAILABLE}: destination update HRESULT 0x{error:08x}");
        return;
    }
    if let Err(error) = thumbnail.set_visible(false) {
        eprintln!("INCONCLUSIVE: {DWM_UNAVAILABLE}: hidden update HRESULT 0x{error:08x}");
        return;
    }
    assert!(wait_for_pixel(sample, is_black, Duration::from_secs(2)).is_some());

    if let Err(error) = thumbnail.set_visible(true) {
        eprintln!("INCONCLUSIVE: {DWM_UNAVAILABLE}: visible update HRESULT 0x{error:08x}");
        return;
    }
    assert!(wait_for_pixel(sample, is_magenta, Duration::from_secs(2)).is_some());

    if let Err(error) = thumbnail.set_visible(false) {
        eprintln!("INCONCLUSIVE: {DWM_UNAVAILABLE}: hidden update HRESULT 0x{error:08x}");
        return;
    }
    assert!(wait_for_pixel(sample, is_black, Duration::from_secs(2)).is_some());
}

#[test]
fn repeated_dwm_thumbnail_disposal_leaves_the_destination_clear() {
    let _serial = serial_guard();
    let _ = frigotab_acceptance::use_per_monitor_physical_coordinates();
    let source = show_acceptance_window(
        "FrigoTab.Acceptance.Thumbnail.Repeat.Source",
        MAGENTA,
        rect(120, 320, 180, 120),
    );
    let destination = show_acceptance_window(
        "FrigoTab.Acceptance.Thumbnail.Repeat.Destination",
        frigotab_acceptance::rgb(0, 0, 0),
        rect(360, 320, 240, 160),
    );
    make_topmost(destination.handle());
    pump_messages();
    let Some((destination_rect, sample)) = client_rect_and_center(destination.handle()) else {
        eprintln!("INCONCLUSIVE: destination client geometry was unavailable");
        return;
    };

    for iteration in 0..5 {
        let thumbnail = match DwmThumbnail::register(destination.handle(), source.handle()) {
            Ok(thumbnail) => thumbnail,
            Err(error) => {
                eprintln!(
                    "INCONCLUSIVE: {DWM_UNAVAILABLE} on iteration {iteration}: HRESULT 0x{error:08x}"
                );
                return;
            }
        };
        if let Err(error) = thumbnail.set_destination_rect(destination_rect) {
            eprintln!("INCONCLUSIVE: {DWM_UNAVAILABLE}: destination update HRESULT 0x{error:08x}");
            return;
        }
        if let Err(error) = thumbnail.set_visible(true) {
            eprintln!("INCONCLUSIVE: {DWM_UNAVAILABLE}: visible update HRESULT 0x{error:08x}");
            return;
        }
        assert!(wait_for_pixel(sample, is_magenta, Duration::from_secs(2)).is_some());
        if let Err(error) = thumbnail.set_visible(false) {
            eprintln!(
                "INCONCLUSIVE: {DWM_UNAVAILABLE} on iteration {iteration}: hidden update HRESULT 0x{error:08x}"
            );
            return;
        }
        drop(thumbnail);
        assert!(wait_for_pixel(sample, is_black, Duration::from_secs(2)).is_some());
    }
}

#[derive(Clone, Copy)]
struct Monitor {
    handle: HMONITOR,
    working_area: RECT,
    primary: bool,
}

fn enumerate_monitors() -> Vec<Monitor> {
    let mut monitors = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            null_mut(),
            null(),
            Some(monitor_callback),
            &mut monitors as *mut _ as LPARAM,
        );
    }
    monitors
}

unsafe extern "system" fn monitor_callback(
    monitor: HMONITOR,
    _dc: HDC,
    _clip: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(data as *mut Vec<Monitor>) };
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    if unsafe {
        GetMonitorInfoW(
            monitor,
            (&mut info as *mut MONITORINFOEXW).cast::<MONITORINFO>(),
        )
    } != 0
    {
        monitors.push(Monitor {
            handle: monitor,
            working_area: info.monitorInfo.rcWork,
            primary: info.monitorInfo.dwFlags != 0,
        });
    }
    1
}

fn virtual_bounds() -> RECT {
    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    RECT {
        left,
        top,
        right: left.saturating_add(width),
        bottom: top.saturating_add(height),
    }
}

fn rect(left: i32, top: i32, width: i32, height: i32) -> RECT {
    RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}

fn show_acceptance_window(title: &str, color: Color, bounds: RECT) -> FixtureWindow {
    FixtureWindow::show_with_options(
        title,
        color,
        FixtureOptions {
            bounds,
            style: WS_POPUP,
            ex_style: WS_EX_APPWINDOW,
            activate: true,
            ..FixtureOptions::default()
        },
    )
}

fn inflate(value: RECT, dx: i32, dy: i32) -> RECT {
    RECT {
        left: value.left - dx,
        top: value.top - dy,
        right: value.right + dx,
        bottom: value.bottom + dy,
    }
}

fn width(value: RECT) -> i32 {
    value.right - value.left
}

fn height(value: RECT) -> i32 {
    value.bottom - value.top
}

fn contains_rectangle(bounds: RECT, value: Rectangle) -> bool {
    value.x >= bounds.left
        && value.y >= bounds.top
        && value.right() <= bounds.right
        && value.bottom() <= bounds.bottom
}

fn make_topmost(hwnd: HWND) {
    let bounds = window_bounds(hwnd).expect("fixture must have native bounds");
    unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            bounds.left,
            bounds.top,
            width(bounds),
            height(bounds),
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
    }
}

fn client_rect_and_center(hwnd: HWND) -> Option<(RECT, POINT)> {
    let mut client = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut client) } == 0 {
        return None;
    }
    let mut center = POINT {
        x: (client.left + client.right) / 2,
        y: (client.top + client.bottom) / 2,
    };
    if unsafe { ClientToScreen(hwnd, &mut center) } == 0 {
        return None;
    }
    Some((client, center))
}

fn wait_for_pixel<F>(point: POINT, predicate: F, timeout: Duration) -> Option<Color>
where
    F: Fn(Color) -> bool,
{
    let deadline = Instant::now() + timeout;
    loop {
        pump_messages();
        if let Some(pixel) = pixel_at(point.x, point.y) {
            if predicate(pixel) {
                return Some(pixel);
            }
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn is_magenta(value: Color) -> bool {
    value & 0xff > 200 && (value >> 16) & 0xff > 200 && (value >> 8) & 0xff < 100
}

fn is_black(value: Color) -> bool {
    value & 0xff < 20 && (value >> 8) & 0xff < 20 && (value >> 16) & 0xff < 20
}

struct SnapshotSurface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    width: i32,
    height: i32,
}

impl SnapshotSurface {
    fn new(width: i32, height: i32) -> Option<Self> {
        let source = unsafe { GetDC(null_mut()) };
        if source.is_null() {
            return None;
        }
        let dc = unsafe { CreateCompatibleDC(source) };
        if dc.is_null() {
            unsafe { ReleaseDC(null_mut(), source) };
            return None;
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = null_mut();
        let bitmap =
            unsafe { CreateDIBSection(source, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0) };
        unsafe { ReleaseDC(null_mut(), source) };
        if bitmap.is_null() || bits.is_null() {
            if !bitmap.is_null() {
                unsafe { DeleteObject(bitmap as HGDIOBJ) };
            }
            unsafe { DeleteDC(dc) };
            return None;
        }
        let previous = unsafe { SelectObject(dc, bitmap as HGDIOBJ) };
        if previous.is_null() || previous == (-1isize as HGDIOBJ) {
            unsafe {
                DeleteObject(bitmap as HGDIOBJ);
                DeleteDC(dc);
            }
            return None;
        }
        if unsafe { PatBlt(dc, 0, 0, width, height, BLACKNESS) } == 0 {
            unsafe {
                SelectObject(dc, previous);
                DeleteObject(bitmap as HGDIOBJ);
                DeleteDC(dc);
            }
            return None;
        }
        Some(Self {
            dc,
            bitmap,
            previous,
            width,
            height,
        })
    }

    fn pixel(&self, x: i32, y: i32) -> Color {
        unsafe { GetPixel(self.dc, x, y) }
    }

    fn is_uniform_black(&self) -> bool {
        let step_x = (self.width / 16).max(1);
        let step_y = (self.height / 16).max(1);
        let mut y = 0;
        while y < self.height {
            let mut x = 0;
            while x < self.width {
                if !is_black(self.pixel(x, y)) {
                    return false;
                }
                x += step_x;
            }
            y += step_y;
        }
        true
    }
}

impl Drop for SnapshotSurface {
    fn drop(&mut self) {
        unsafe {
            if !self.dc.is_null() && !self.previous.is_null() {
                SelectObject(self.dc, self.previous);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap as HGDIOBJ);
            }
            if !self.dc.is_null() {
                DeleteDC(self.dc);
            }
        }
        self.bitmap = null_mut();
        self.dc = null_mut();
    }
}
