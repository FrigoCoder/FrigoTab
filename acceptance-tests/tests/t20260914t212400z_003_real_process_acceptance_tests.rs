#![cfg(windows)]

//! Black-box acceptance scenarios for the real FrigoTab executable.
//!
//! The timestamp is part of the test module/file identity.  The test bodies
//! intentionally use real HWNDs, real process state, and real screen pixels;
//! there is no substitute switcher model or synthetic protocol here.

use std::mem::{size_of, zeroed};
use std::process::{Child, Command, ExitStatus};
use std::ptr::null_mut;
use std::thread;
use std::time::{Duration, Instant};

use frigotab::desktop::ShellDesktopSnapshot;
use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, GREEN, MAGENTA, RunningFrigoTab, capture_screen_image,
    color_distance, pixel_at, pump_messages, serial_guard, set_per_monitor_dpi_awareness,
    visible_owned_layered_windows, wait_until, window_bounds,
};
use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDC, HGDIOBJ, ReleaseDC, SelectObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    HWND_NOTOPMOST, PostMessageW, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    WM_CLOSE, WS_EX_TOPMOST, WS_POPUP,
};

const MARKED_ALT_KEY: u16 = 0xa4;
const MARKED_TAB_KEY: u16 = 0x09;

#[test]
fn executable_starts_resident_without_showing_a_window() {
    let _desktop = serial_guard();
    set_per_monitor_dpi_awareness();
    let mut application = RunningFrigoTab::start().expect("FrigoTab did not start");

    assert!(!application.has_exited().expect("could not query FrigoTab"));
    assert!(
        !application.windows().is_empty(),
        "the executable did not create its native message window"
    );
    assert!(
        application.visible_windows().is_empty(),
        "the resident executable exposed a visible window"
    );
}

#[test]
fn a_second_executable_instance_exits_while_the_first_remains_resident() {
    let _desktop = serial_guard();
    set_per_monitor_dpi_awareness();
    let mut first = RunningFrigoTab::start().expect("the first FrigoTab did not start");
    let executable = RunningFrigoTab::executable_path().expect("FrigoTab.exe was not built");
    let mut second = Command::new(executable)
        .spawn()
        .expect("the competing FrigoTab could not be started");

    let status = wait_for_child(&mut second, Duration::from_secs(5))
        .expect("the competing executable did not exit");
    assert_eq!(status.code(), Some(0), "the competing instance failed");
    assert!(!first.has_exited().expect("could not query first FrigoTab"));
}

#[test]
fn closing_the_real_session_window_stops_the_tray_process() {
    let _desktop = serial_guard();
    set_per_monitor_dpi_awareness();
    let mut application = RunningFrigoTab::start().expect("FrigoTab did not start");
    let owner = application.session_window();

    assert!(
        unsafe { PostMessageW(owner, WM_CLOSE, 0, 0) } != 0,
        "WM_CLOSE could not be posted to the real owner window"
    );
    let status = application
        .wait_exit(Duration::from_secs(5))
        .expect("closing the native session window did not stop FrigoTab");
    assert_eq!(status.code(), Some(0));
}

#[test]
fn executable_opens_a_real_full_desktop_session() {
    let _desktop = serial_guard();
    set_per_monitor_dpi_awareness();
    let fixture = FixtureWindow::show_with_options(
        "FrigoTab process acceptance fixture",
        GREEN,
        FixtureOptions {
            bounds: RECT {
                left: 80,
                top: 80,
                right: 720,
                bottom: 560,
            },
            style: WS_POPUP,
            activate: true,
            ..FixtureOptions::default()
        },
    );
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");

    open_with_marked_alt_tab(&application);
    let bounds = application.bounds();
    let expected = virtual_desktop_bounds();
    assert_rect_eq(
        bounds,
        RECT {
            left: expected.left,
            top: expected.top,
            right: expected.right,
            bottom: expected.bottom,
        },
        "the session owner does not cover the virtual desktop",
    );

    assert!(
        wait_for_lime_preview(&application, Duration::from_secs(5)).is_some(),
        "the executable did not render the real fixture preview"
    );
    drop(fixture);
}

#[test]
fn pointer_click_on_a_real_preview_closes_the_executable_session() {
    let _desktop = serial_guard();
    set_per_monitor_dpi_awareness();
    let fixture = FixtureWindow::show_with_options(
        "FrigoTab pointer acceptance fixture",
        GREEN,
        FixtureOptions {
            bounds: RECT {
                left: 80,
                top: 80,
                right: 720,
                bottom: 560,
            },
            style: WS_POPUP,
            activate: true,
            ..FixtureOptions::default()
        },
    );
    let mut application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    open_with_marked_alt_tab(&application);

    let point = wait_for_lime_preview(&application, Duration::from_secs(5))
        .expect("the real fixture preview did not become visible");
    assert!(
        application.click_at(point.x, point.y),
        "the real pointer click could not be sent through the system input stream"
    );

    assert!(
        wait_until(Duration::from_secs(5), || {
            application.visible_windows().is_empty()
        }),
        "the real pointer click did not close the session"
    );
    assert!(!application.has_exited().expect("could not query FrigoTab"));
    drop(fixture);
}

#[test]
fn first_visible_frame_uses_the_desktop_instead_of_the_covering_application() {
    let _desktop = serial_guard();
    set_per_monitor_dpi_awareness();
    let desktop = virtual_desktop_bounds();
    let primary = RECT {
        left: 0,
        top: 0,
        right: unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(SM_CXSCREEN)
        },
        bottom: unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(SM_CYSCREEN)
        },
    };

    // Preserve the unobstructed corner pixel before introducing the sentinel.
    // The sample is chosen from the four margin corners, matching the original
    // acceptance scenario and staying outside the preview grid's 0.5% margin.
    let expected =
        capture_shell_desktop(desktop).expect("Explorer did not expose a desktop surface");
    let (sample, desktop_pixel) = desktop_sample(&expected, primary, desktop)
        .expect("no stable desktop corner was available");

    let fixture = FixtureWindow::show_with_options(
        "FrigoTab desktop backdrop sentinel",
        MAGENTA,
        FixtureOptions {
            bounds: primary,
            style: WS_POPUP,
            ex_style: WS_EX_TOPMOST,
            activate: true,
            ..FixtureOptions::default()
        },
    );
    unsafe {
        DwmFlush();
    }
    let sentinel_center = POINT {
        x: (primary.left + primary.right) / 2,
        y: (primary.top + primary.bottom) / 2,
    };
    assert!(wait_until(Duration::from_secs(2), || {
        pixel_at(sentinel_center.x, sentinel_center.y)
            .map(|color| color_distance(color, MAGENTA) <= 10)
            .unwrap_or(false)
    }));

    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    // Keep the sentinel visible while moving it below FrigoTab's future
    // topmost owner. A screen-copy implementation would retain its pixels;
    // an Explorer desktop capture will not.
    assert!(
        unsafe {
            SetWindowPos(
                fixture.handle(),
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        } != 0
    );
    pump_messages();

    open_with_marked_alt_tab(&application);
    unsafe {
        DwmFlush();
    }
    let first_frame = pixel_at(sample.x, sample.y).expect("could not read first frame pixel");
    assert!(
        color_distance(desktop_pixel, first_frame) <= 45,
        "the first frame copied application pixels: desktop={desktop_pixel:#08x}, first={first_frame:#08x}"
    );
    thread::sleep(Duration::from_millis(300));
    unsafe {
        DwmFlush();
    }
    let settled_frame = pixel_at(sample.x, sample.y).expect("could not read settled frame pixel");
    assert!(
        color_distance(first_frame, settled_frame) <= 20,
        "the background changed after opening: first={first_frame:#08x}, settled={settled_frame:#08x}"
    );
    drop(fixture);
}

fn open_with_marked_alt_tab(application: &RunningFrigoTab) {
    // RunningFrigoTab's responsive-owner barrier completes only after the
    // production hook has been installed and the UI message loop is running.
    let alt_down = application.send_marked_test_key(MARKED_ALT_KEY, false);
    let tab_down = application.send_marked_test_key(MARKED_TAB_KEY, false);
    let opened = alt_down && tab_down && application.wait_visible(Duration::from_secs(5));
    // Complete the physical chord and balance both key-up transitions. The
    // default Sticky behavior intentionally leaves the session visible after
    // Alt-up, so later assertions still observe the open session.
    let tab_up = application.send_marked_test_key(MARKED_TAB_KEY, true);
    let alt_up = application.send_marked_test_key(MARKED_ALT_KEY, true);
    assert!(alt_down, "marked Alt-down was rejected");
    assert!(tab_down, "marked Tab-down was rejected");
    assert!(tab_up, "marked Tab-up was rejected");
    assert!(alt_up, "marked Alt-up was rejected");
    assert!(
        opened,
        "the marked Alt+Tab chord did not open the real session"
    );
}

fn wait_for_child(child: &mut Child, timeout: Duration) -> Option<ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().expect("could not query child process") {
            return Some(status);
        }
        pump_messages();
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_lime_preview(application: &RunningFrigoTab, timeout: Duration) -> Option<POINT> {
    let deadline = Instant::now() + timeout;
    loop {
        let point = visible_owned_layered_windows(application.owner())
            .into_iter()
            .filter_map(|overlay| {
                let bounds = window_bounds(overlay)?;
                let image = capture_screen_image(bounds)?;
                let selected_green = 0x007f_7f00;
                let samples = (bounds.top..bounds.bottom)
                    .step_by(4)
                    .flat_map(|y| (bounds.left..bounds.right).step_by(4).map(move |x| (x, y)))
                    .filter(|&(x, y)| {
                        image.pixel(x, y).is_some_and(|pixel| {
                            color_distance(pixel, GREEN) <= 55
                                || color_distance(pixel, selected_green) <= 55
                        })
                    })
                    .count();
                Some((samples, bounds))
            })
            .max_by_key(|(samples, _)| *samples)
            .filter(|(samples, _)| *samples >= 100)
            .map(|(_, bounds)| POINT {
                x: bounds.left + (bounds.right - bounds.left) / 2,
                y: bounds.top + (bounds.bottom - bounds.top) / 2,
            });
        if point.is_some() {
            return point;
        }
        pump_messages();
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

struct ShellImage {
    bounds: RECT,
    pixels: Vec<u32>,
}

impl ShellImage {
    fn pixel(&self, x: i32, y: i32) -> Option<u32> {
        if x < self.bounds.left
            || x >= self.bounds.right
            || y < self.bounds.top
            || y >= self.bounds.bottom
        {
            return None;
        }
        let width = (self.bounds.right - self.bounds.left) as usize;
        let index = (y - self.bounds.top) as usize * width + (x - self.bounds.left) as usize;
        self.pixels.get(index).copied()
    }
}

fn capture_shell_desktop(bounds: RECT) -> Option<ShellImage> {
    let snapshot = ShellDesktopSnapshot::capture(bounds);
    if !snapshot.is_available() {
        return None;
    }
    let width = bounds.right - bounds.left;
    let height = bounds.bottom - bounds.top;
    if width <= 0 || height <= 0 {
        return None;
    }
    let screen = unsafe { GetDC(null_mut()) };
    if screen.is_null() {
        return None;
    }
    let memory = unsafe { CreateCompatibleDC(screen) };
    if memory.is_null() {
        unsafe { ReleaseDC(null_mut(), screen) };
        return None;
    }
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            ..unsafe { zeroed() }
        },
        ..unsafe { zeroed() }
    };
    let mut bits = null_mut();
    let bitmap =
        unsafe { CreateDIBSection(screen, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0) };
    if bitmap.is_null() || bits.is_null() {
        if !bitmap.is_null() {
            unsafe { DeleteObject(bitmap) };
        }
        unsafe {
            DeleteDC(memory);
            ReleaseDC(null_mut(), screen);
        }
        return None;
    }
    let previous = unsafe { SelectObject(memory, bitmap as HGDIOBJ) };
    snapshot.draw(
        memory,
        RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        },
    );
    // A 32-bpp DIB exposes BGRA bytes (u32 AARRGGBB), whereas GetPixel and
    // the rest of this harness use COLORREF (u32 00BBGGRR).  Bitmap.GetPixel
    // performed this conversion in the original acceptance scenario.
    let pixels = unsafe {
        std::slice::from_raw_parts(bits as *const u32, (width * height) as usize)
            .iter()
            .map(|pixel| {
                let red = (pixel >> 16) & 0xff;
                let green = (pixel >> 8) & 0xff;
                let blue = pixel & 0xff;
                red | (green << 8) | (blue << 16)
            })
            .collect()
    };
    unsafe {
        if !previous.is_null() {
            SelectObject(memory, previous);
        }
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(null_mut(), screen);
    }
    Some(ShellImage { bounds, pixels })
}

fn desktop_sample(image: &ShellImage, monitor: RECT, desktop: RECT) -> Option<(POINT, u32)> {
    const INSET: i32 = 2;
    let candidates = [
        POINT {
            x: monitor.left + INSET,
            y: monitor.top + INSET,
        },
        POINT {
            x: monitor.right - INSET - 1,
            y: monitor.top + INSET,
        },
        POINT {
            x: monitor.left + INSET,
            y: monitor.bottom - INSET - 1,
        },
        POINT {
            x: monitor.right - INSET - 1,
            y: monitor.bottom - INSET - 1,
        },
    ];
    candidates.into_iter().find_map(|point| {
        let color = image.pixel(point.x, point.y)?;
        // The selected point must be inside the captured virtual desktop and
        // cannot be the sentinel colour already present in the background.
        if point.x >= desktop.left
            && point.x < desktop.right
            && point.y >= desktop.top
            && point.y < desktop.bottom
            && !is_magenta(color)
        {
            Some((point, color))
        } else {
            None
        }
    })
}

fn is_magenta(color: u32) -> bool {
    color & 0xff > 200 && (color >> 16) & 0xff > 200 && (color >> 8) & 0xff < 80
}

fn virtual_desktop_bounds() -> RECT {
    unsafe {
        let left = windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(SM_XVIRTUALSCREEN);
        let top = windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(SM_YVIRTUALSCREEN);
        RECT {
            left,
            top,
            right: left
                + windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(SM_CXVIRTUALSCREEN),
            bottom: top
                + windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(SM_CYVIRTUALSCREEN),
        }
    }
}

fn assert_rect_eq(actual: RECT, expected: RECT, message: &str) {
    assert!(
        actual.left == expected.left
            && actual.top == expected.top
            && actual.right == expected.right
            && actual.bottom == expected.bottom,
        "{message}: actual=({}, {}, {}, {}), expected=({}, {}, {}, {})",
        actual.left,
        actual.top,
        actual.right,
        actual.bottom,
        expected.left,
        expected.top,
        expected.right,
        expected.bottom
    );
}
