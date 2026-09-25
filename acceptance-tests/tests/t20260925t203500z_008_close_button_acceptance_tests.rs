#![cfg(windows)]

//! Real acceptance coverage for the optional close button on each preview.
//!
//! These scenarios use the live Win32 session and fixture HWNDs.  The close
//! button is deliberately exercised through the same screen-space pointer
//! path as the application, while source lifetime and foreground ownership
//! are observed through native handles.

use std::time::Duration;

use frigotab::geometry::{Layout, ScreenPoint};
use frigotab::input::{KeyHandling, KeyTransition, KeyboardInput, SwitcherKey};
use frigotab::switcher::SwitcherState;
use frigotab::window::{WindowFinder, WindowHandle};
use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, LiveSession, LiveTile, MAGENTA, RunningFrigoTab, ScreenCapture,
    capture_screen_image, color_distance, foreground_window, is_window_visible, pump_messages,
    screen_pixel, serial_guard, set_per_monitor_dpi_awareness, wait_until,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IsWindow, PostMessageW, WM_MOUSEMOVE, WS_EX_APPWINDOW, WS_POPUP,
};

const TIMEOUT: Duration = Duration::from_secs(5);
const CLOSE_BUTTON_INSET: i32 = 8;
const CLOSE_BUTTON_SIZE: i32 = 32;

#[test]
fn close_button_is_enabled_by_default_and_closes_exact_source_without_activation() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let mut session = LiveSession::new();

    assert!(session.close_buttons_visible());
    open_session(&mut session);
    assert!(wait_until(TIMEOUT, || foreground_window() == session.owner()));

    let target = fixture_tiles(&session)
        .into_iter()
        .nth(1)
        .expect("a second real fixture preview is required");
    let other_source = fixture_tiles(&session)
        .into_iter()
        .find(|tile| tile.source != target.source)
        .expect("another real fixture preview is required")
        .source;

    assert!(wait_until(TIMEOUT, || close_button_is_visible(target)));
    let button = close_button_center(target);
    session.mouse_move(button);
    assert_eq!(Some(target.source), session.selected_source());
    session.mouse_click(button);

    assert!(
        wait_until(TIMEOUT, || !is_valid_window(target.source)),
        "the close button did not close its source HWND"
    );
    assert!(
        session.is_visible(),
        "closing a source must keep the session open"
    );
    assert_eq!(
        None,
        session.selected_index(),
        "closing a source must clear the stale selection"
    );
    assert_eq!(
        session.owner(),
        foreground_window(),
        "closing a source must not activate it"
    );
    assert!(
        is_valid_window(other_source),
        "closing one preview unexpectedly closed another source HWND"
    );

    assert_eq!(
        KeyHandling::Consume,
        session.key(KeyboardInput::new(
            SwitcherKey::Escape,
            KeyTransition::Down,
            false,
            false,
            false,
        ))
    );
    assert!(wait_until(TIMEOUT, || {
        session.switcher_state() == SwitcherState::Idle && !is_window_visible(session.owner())
    }));
}

#[test]
fn hidden_close_buttons_leave_the_same_point_as_normal_tile_activation() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let mut session = LiveSession::new();
    session.set_close_buttons_visible(false);

    assert!(!session.close_buttons_visible());
    open_session(&mut session);
    let target = fixture_tiles(&session)
        .into_iter()
        .nth(1)
        .expect("a second real fixture preview is required");
    let button = close_button_center(target);

    session.mouse_move(button);
    session.mouse_click(button);

    assert!(
        wait_until(TIMEOUT, || !session.is_visible()),
        "a click with close buttons hidden did not follow normal tile activation"
    );
    assert!(
        is_valid_window(target.source),
        "the hidden close-button setting closed the source instead of activating it"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == target.source),
        "normal tile activation did not foreground the selected source"
    );
}

#[test]
fn close_buttons_toggle_from_the_real_tray_menu_and_update_a_real_session() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let application = RunningFrigoTab::start().expect("FrigoTab did not start");

    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be inspected");
    assert_checked_top_level(&menu, "Close buttons");

    assert!(application.select_top_level_tray_item("Close buttons"));
    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be reopened after hiding close buttons");
    assert_unchecked_top_level(&menu, "Close buttons");

    let fixture = FixtureWindow::show_with_options(
        "FrigoTab tray close-button fixture",
        MAGENTA,
        FixtureOptions {
            bounds: RECT {
                left: 180,
                top: 140,
                right: 820,
                bottom: 620,
            },
            style: WS_POPUP,
            ex_style: WS_EX_APPWINDOW,
            activate: true,
            ..FixtureOptions::default()
        },
    );
    pump_messages();
    let layout = Layout::new(&WindowFinder::new().windows);
    let bounds = layout
        .bounds
        .get(&WindowHandle::new(fixture.handle()))
        .copied()
        .map(|bounds| RECT {
            left: bounds.x,
            top: bounds.y,
            right: bounds.right(),
            bottom: bounds.bottom(),
        })
        .expect("the real fixture should have production tile bounds");

    assert!(application.open(), "the real session did not accept open");
    assert!(application.wait_visible(TIMEOUT));
    post_owner_mouse_point(application.owner(), 1, 1);
    assert!(
        wait_until(TIMEOUT, || tile_has_color(bounds, MAGENTA)),
        "the solid real fixture did not appear in its DWM tile"
    );
    assert!(
        !close_button_is_visible_in(bounds),
        "the tray-disabled close button was still drawn"
    );

    assert!(application.select_top_level_tray_item("Close buttons"));
    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be reopened after showing close buttons");
    assert_checked_top_level(&menu, "Close buttons");
    assert!(
        wait_until(TIMEOUT, || close_button_is_visible_in(bounds)),
        "the tray-enabled close button was not drawn in the active real session"
    );
}

fn open_session(session: &mut LiveSession) {
    assert_eq!(KeyHandling::Consume, session.open());
    assert!(
        session.wait_visible(TIMEOUT),
        "the real session did not become visible"
    );
    assert!(
        !session.tiles().is_empty(),
        "the real session has no preview HWNDs"
    );
}

fn fixture_tiles(session: &LiveSession) -> Vec<LiveTile> {
    let fixtures = session.fixture_handles();
    session
        .tiles()
        .into_iter()
        .filter(|tile| fixtures.contains(&tile.source))
        .collect()
}

fn close_button_center(tile: LiveTile) -> ScreenPoint {
    ScreenPoint::new(
        tile.bounds.right - CLOSE_BUTTON_INSET - CLOSE_BUTTON_SIZE / 2,
        tile.bounds.top + CLOSE_BUTTON_INSET + CLOSE_BUTTON_SIZE / 2,
    )
}

fn close_button_rect(bounds: RECT) -> RECT {
    RECT {
        left: bounds.right - CLOSE_BUTTON_INSET - CLOSE_BUTTON_SIZE,
        top: bounds.top + CLOSE_BUTTON_INSET,
        right: bounds.right - CLOSE_BUTTON_INSET,
        bottom: bounds.top + CLOSE_BUTTON_INSET + CLOSE_BUTTON_SIZE,
    }
}

fn close_button_is_visible(tile: LiveTile) -> bool {
    close_button_is_visible_in(tile.bounds)
}

fn close_button_is_visible_in(bounds: RECT) -> bool {
    let Some(image) = capture_screen_image(bounds) else {
        return false;
    };
    let area = close_button_rect(bounds);
    let dark_pixels = count_pixels(&image, area, is_black);
    let white_pixels = count_pixels(&image, area, is_white);
    dark_pixels >= 100 && white_pixels >= 8
}

fn tile_has_color(bounds: RECT, expected: u32) -> bool {
    unsafe {
        let _ = DwmFlush();
    }
    let x = bounds.left + (bounds.right - bounds.left) * 3 / 4;
    let y = bounds.top + (bounds.bottom - bounds.top) * 3 / 4;
    screen_pixel(x, y).is_some_and(|actual| color_distance(actual, expected) <= 55)
}

fn post_owner_mouse_point(owner: HWND, x: i32, y: i32) {
    let packed = ((y as i16 as u16 as u32) << 16) | (x as i16 as u16 as u32);
    assert!(unsafe { PostMessageW(owner, WM_MOUSEMOVE, 0, packed as LPARAM) } != 0);
    pump_messages();
}

fn count_pixels<F>(image: &ScreenCapture, bounds: RECT, predicate: F) -> usize
where
    F: Fn(u32) -> bool,
{
    (bounds.top..bounds.bottom)
        .flat_map(|y| (bounds.left..bounds.right).map(move |x| (x, y)))
        .filter(|&(x, y)| image.pixel(x, y).is_some_and(&predicate))
        .count()
}

fn is_black(color: u32) -> bool {
    color & 0xff <= 35 && (color >> 8) & 0xff <= 35 && (color >> 16) & 0xff <= 35
}

fn is_white(color: u32) -> bool {
    color & 0xff >= 200 && (color >> 8) & 0xff >= 200 && (color >> 16) & 0xff >= 200
}

fn is_valid_window(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 }
}

fn assert_checked_top_level(menu: &[frigotab_acceptance::TrayMenuItem], label: &str) {
    let item = menu
        .iter()
        .find(|item| item.label == label)
        .unwrap_or_else(|| panic!("tray item {label} was not present"));
    assert!(item.checked, "tray item {label} was not checked");
}

fn assert_unchecked_top_level(menu: &[frigotab_acceptance::TrayMenuItem], label: &str) {
    let item = menu
        .iter()
        .find(|item| item.label == label)
        .unwrap_or_else(|| panic!("tray item {label} was not present"));
    assert!(!item.checked, "tray item {label} was unexpectedly checked");
}
