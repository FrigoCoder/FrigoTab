#![cfg(windows)]

//! Black-box acceptance checks for the real tray settings menu and backdrop
//! modes. The menu is opened through the production notification callback;
//! tests never send the production command IDs directly.

use std::time::Duration;

use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, MAGENTA, RunningFrigoTab, ScreenCapture, TrayMenuItem,
    capture_screen_image, color_distance, pump_messages, screen_pixel, serial_guard,
    set_per_monitor_dpi_awareness, visible_owned_layered_windows, wait_until, window_bounds,
};
use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::Graphics::Gdi::{GetDC, PaintDesktop, ReleaseDC};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    WS_EX_APPWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn real_tray_menu_checks_sticky_and_full_desktop_by_default() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let application = RunningFrigoTab::start().expect("FrigoTab did not start");

    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be inspected");
    assert_checked_child(&menu, "Alt-Tab behavior", "Sticky");
    assert_unchecked_child(&menu, "Alt-Tab behavior", "Tap (classic)");
    assert_checked_child(&menu, "Background", "Full desktop");
    assert_unchecked_child(&menu, "Background", "Background image only");
    assert_unchecked_child(&menu, "Background", "Black rectangle");
}

#[test]
fn selecting_tap_from_the_real_tray_menu_updates_the_visible_checkmark() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let application = RunningFrigoTab::start().expect("FrigoTab did not start");

    assert!(application.select_tray_menu_item("Alt-Tab behavior", "Tap (classic)"));
    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be reopened");
    assert_checked_child(&menu, "Alt-Tab behavior", "Tap (classic)");
    assert_unchecked_child(&menu, "Alt-Tab behavior", "Sticky");

    assert!(application.select_tray_menu_item("Alt-Tab behavior", "Sticky"));
    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be reopened after selecting Sticky");
    assert_checked_child(&menu, "Alt-Tab behavior", "Sticky");
    assert_unchecked_child(&menu, "Alt-Tab behavior", "Tap (classic)");
}

#[test]
fn selecting_black_from_the_real_tray_menu_paints_a_black_next_session() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let fixture = full_desktop_fixture("FrigoTab black backdrop fixture");
    let application = RunningFrigoTab::start().expect("FrigoTab did not start");

    assert!(application.select_tray_menu_item("Background", "Black rectangle"));
    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be reopened");
    assert_checked_child(&menu, "Background", "Black rectangle");
    assert!(application.open(), "the real session did not accept open");
    assert!(application.wait_visible(TIMEOUT));

    let sample_points = background_sample_points(&application);
    assert!(
        !sample_points.is_empty(),
        "no point outside the real preview tiles"
    );
    assert!(wait_until(TIMEOUT, || {
        flush_composition();
        sample_points
            .iter()
            .all(|point| screen_pixel(point.x, point.y).is_some_and(is_black))
    }));

    assert!(application.select_tray_menu_item("Background", "Full desktop"));
    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be reopened after selecting Full desktop");
    assert_checked_child(&menu, "Background", "Full desktop");
    assert_unchecked_child(&menu, "Background", "Black rectangle");

    drop(fixture);
}

#[test]
fn selecting_image_only_from_the_real_tray_menu_hides_fixture_pixels() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let painted_desktop = capture_painted_desktop(virtual_desktop_bounds());
    let fixture = full_desktop_fixture("FrigoTab image-only backdrop fixture");
    let application = RunningFrigoTab::start().expect("FrigoTab did not start");

    assert!(application.select_tray_menu_item("Background", "Background image only"));
    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be reopened");
    assert_checked_child(&menu, "Background", "Background image only");
    assert!(application.open(), "the real session did not accept open");
    assert!(application.wait_visible(TIMEOUT));

    let sample_points = background_sample_points(&application);
    assert!(
        !sample_points.is_empty(),
        "no point outside the real preview tiles"
    );
    assert!(
        wait_until(TIMEOUT, || {
            flush_composition();
            sample_points.iter().all(|point| {
                painted_desktop
                    .pixel(point.x, point.y)
                    .is_some_and(|expected| {
                        screen_pixel(point.x, point.y)
                            .is_some_and(|actual| color_distance(actual, expected) <= 45)
                    })
            })
        }),
        "the image-only backdrop did not match Windows' painted desktop"
    );

    drop(fixture);
}

/// Paint a temporary topmost window using the same User32 operation as the
/// production image-only mode, then retain its composed pixels for comparison.
fn capture_painted_desktop(bounds: RECT) -> ScreenCapture {
    let reference = FixtureWindow::show_with_options(
        "FrigoTab PaintDesktop reference",
        0,
        FixtureOptions {
            bounds,
            style: WS_POPUP,
            ex_style: WS_EX_TOPMOST,
            activate: false,
            ..FixtureOptions::default()
        },
    );
    pump_messages();
    let dc = unsafe { GetDC(reference.handle()) };
    assert!(
        !dc.is_null(),
        "could not acquire the PaintDesktop reference DC"
    );
    unsafe {
        let _ = PaintDesktop(dc);
        ReleaseDC(reference.handle(), dc);
    }
    flush_composition();
    let capture = capture_screen_image(bounds).expect("could not capture the painted desktop");
    drop(reference);
    pump_messages();
    capture
}

fn assert_checked_child(menu: &[TrayMenuItem], parent: &str, child: &str) {
    let item = child_item(menu, parent, child);
    assert!(
        item.checked,
        "tray item {parent} -> {child} was not checked"
    );
}

fn assert_unchecked_child(menu: &[TrayMenuItem], parent: &str, child: &str) {
    let item = child_item(menu, parent, child);
    assert!(
        !item.checked,
        "tray item {parent} -> {child} was unexpectedly checked"
    );
}

fn child_item<'a>(menu: &'a [TrayMenuItem], parent: &str, child: &str) -> &'a TrayMenuItem {
    menu.iter()
        .find(|item| item.label == parent)
        .unwrap_or_else(|| panic!("tray submenu {parent} was not present"))
        .children
        .iter()
        .find(|item| item.label == child)
        .unwrap_or_else(|| panic!("tray item {parent} -> {child} was not present"))
}

fn full_desktop_fixture(title: &str) -> FixtureWindow {
    let bounds = virtual_desktop_bounds();
    let fixture = FixtureWindow::show_with_options(
        title,
        MAGENTA,
        FixtureOptions {
            bounds,
            style: WS_POPUP,
            ex_style: WS_EX_APPWINDOW,
            activate: true,
            ..FixtureOptions::default()
        },
    );
    pump_messages();
    fixture
}

fn background_sample_points(application: &RunningFrigoTab) -> Vec<POINT> {
    let bounds = application.bounds();
    let tiles = visible_owned_layered_windows(application.owner())
        .into_iter()
        .filter_map(window_bounds)
        .collect::<Vec<_>>();
    let candidates = [
        POINT {
            x: bounds.left + 2,
            y: bounds.top + 2,
        },
        POINT {
            x: bounds.right - 3,
            y: bounds.top + 2,
        },
        POINT {
            x: bounds.left + 2,
            y: bounds.bottom - 3,
        },
        POINT {
            x: bounds.right - 3,
            y: bounds.bottom - 3,
        },
    ];
    candidates
        .into_iter()
        .filter(|point| {
            // The rectangular virtual-screen bounds can contain holes when
            // monitors form an L shape. Those coordinates have no screen
            // pixel and therefore cannot be backdrop samples.
            screen_pixel(point.x, point.y).is_some()
                && tiles.iter().all(|tile| {
                    point.x < tile.left
                        || point.x >= tile.right
                        || point.y < tile.top
                        || point.y >= tile.bottom
                })
        })
        .collect()
}

fn virtual_desktop_bounds() -> RECT {
    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    RECT {
        left,
        top,
        right: left + unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) },
        bottom: top + unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) },
    }
}

fn flush_composition() {
    unsafe {
        let _ = DwmFlush();
    }
}

fn is_black(color: u32) -> bool {
    color & 0xff <= 8 && (color >> 8) & 0xff <= 8 && (color >> 16) & 0xff <= 8
}
