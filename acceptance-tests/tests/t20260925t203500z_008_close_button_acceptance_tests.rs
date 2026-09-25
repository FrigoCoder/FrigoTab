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
use frigotab::switcher::{CloseButtonMode, SwitcherState};
use frigotab::window::{WindowFinder, WindowHandle};
use frigotab_acceptance::{
    CursorPosition, FixtureOptions, FixtureWindow, GREEN, LiveSession, LiveTile, MAGENTA,
    RunningFrigoTab, ScreenCapture, capture_screen_image, color_distance, foreground_window,
    is_window_visible, pump_messages, screen_pixel, serial_guard, set_per_monitor_dpi_awareness,
    wait_until,
};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindow, WS_EX_APPWINDOW, WS_POPUP};

const TIMEOUT: Duration = Duration::from_secs(5);
const CLOSE_BUTTON_INSET: i32 = 8;
const CLOSE_BUTTON_SIZE: i32 = 32;

#[test]
fn default_close_button_closes_exact_source_and_refreshes_the_live_layout() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let mut session = LiveSession::new();

    assert_eq!(CloseButtonMode::AlwaysVisible, session.close_button_mode());
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
    let stale_popup = target.popup;

    assert!(wait_until(TIMEOUT, || close_button_is_visible(target)));
    let button = close_button_center(target);
    session.mouse_move(button);
    assert_eq!(Some(target.source), session.selected_source());
    session.mouse_click(button);

    assert!(
        wait_until(TIMEOUT, || {
            !is_valid_window(target.source)
                && !session
                    .tiles()
                    .iter()
                    .any(|tile| tile.source == target.source)
                && session.candidate_count() == session.tiles().len()
        }),
        "the close button did not close its source and refresh the preview graph"
    );
    assert!(
        !is_valid_window(stale_popup),
        "the closed source's stale overlay HWND survived the refresh"
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
    assert_fixture_tiles_match_current_layout(&session);

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
    session.set_close_button_mode(CloseButtonMode::Hidden);

    assert_eq!(CloseButtonMode::Hidden, session.close_button_mode());
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
fn windows_style_button_is_a_transparent_white_x_only_on_the_hovered_thumbnail() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let mut session = LiveSession::new();
    session.set_close_button_mode(CloseButtonMode::HoverOnly);
    let target_fixture = borderless_fixture(
        "FrigoTab hover close-button target",
        MAGENTA,
        RECT {
            left: 180,
            top: 140,
            right: 820,
            bottom: 620,
        },
    );
    let other_fixture = borderless_fixture(
        "FrigoTab hover close-button other",
        GREEN,
        RECT {
            left: 880,
            top: 180,
            right: 1520,
            bottom: 660,
        },
    );
    pump_messages();
    open_session(&mut session);

    let tiles = session.tiles();
    let target = tiles
        .iter()
        .copied()
        .find(|tile| tile.source == target_fixture.handle())
        .expect("the borderless target fixture preview is required");
    let other = tiles
        .iter()
        .copied()
        .find(|tile| tile.source == other_fixture.handle())
        .expect("the other borderless fixture preview is required");

    session.mouse_move(ScreenPoint::new(1, 1));
    assert!(wait_until(TIMEOUT, || {
        close_button_surface_is_hidden(target.bounds)
    }));

    session.mouse_move(tile_center(target));
    assert!(
        wait_until(TIMEOUT, || windows_close_button_is_visible(target.bounds)),
        "hovering a thumbnail did not reveal its transparent white close X"
    );
    assert!(
        close_button_surface_is_hidden(other.bounds),
        "hovering one thumbnail exposed a close X on another thumbnail"
    );

    session.mouse_move(ScreenPoint::new(1, 1));
    assert!(
        wait_until(TIMEOUT, || close_button_surface_is_hidden(target.bounds)),
        "the Windows-style close X remained after the pointer left the thumbnail"
    );

    assert_eq!(
        KeyHandling::Consume,
        session.key(KeyboardInput::new(
            SwitcherKey::Tab,
            KeyTransition::Down,
            true,
            false,
            false,
        ))
    );
    let keyboard_selected = session
        .selected_tile()
        .expect("keyboard navigation should select a real thumbnail");
    assert!(
        close_button_surface_is_hidden(keyboard_selected.bounds),
        "keyboard selection incorrectly acted as pointer hover"
    );

    session.mouse_move(close_button_center(target));
    session.mouse_click(close_button_center(target));
    assert!(wait_until(TIMEOUT, || {
        !is_valid_window(target.source)
            && !session
                .tiles()
                .iter()
                .any(|tile| tile.source == target.source)
    }));
    assert!(session.is_visible());
}

#[test]
fn close_button_modes_are_selectable_from_the_real_tray_and_redraw_a_live_session() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    set_per_monitor_dpi_awareness();
    let application = RunningFrigoTab::start().expect("FrigoTab did not start");

    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be inspected");
    assert_checked_child(&menu, "Close buttons", "Always visible");
    assert_unchecked_child(&menu, "Close buttons", "On hover (Alt-Tab / Win-Tab)");
    assert_unchecked_child(&menu, "Close buttons", "Hidden");

    assert!(application.select_tray_menu_item("Close buttons", "On hover (Alt-Tab / Win-Tab)"));
    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be reopened after changing close-button mode");
    assert_checked_child(&menu, "Close buttons", "On hover (Alt-Tab / Win-Tab)");
    assert_unchecked_child(&menu, "Close buttons", "Always visible");
    assert_unchecked_child(&menu, "Close buttons", "Hidden");

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
    move_pointer(ScreenPoint::new(
        application.bounds().left + 1,
        application.bounds().top + 1,
    ));
    assert!(
        wait_until(TIMEOUT, || tile_has_color(bounds, MAGENTA)),
        "the solid real fixture did not appear in its DWM tile"
    );
    assert!(
        close_button_surface_is_hidden(bounds),
        "the hover-only close button was drawn without pointer hover"
    );

    move_pointer(tile_center_rect(bounds));
    assert!(
        wait_until(TIMEOUT, || windows_close_button_is_visible(bounds)),
        "the tray-selected Windows close-button mode did not react to hover"
    );

    assert!(application.select_tray_menu_item("Close buttons", "Hidden"));
    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be reopened after hiding close buttons");
    assert_checked_child(&menu, "Close buttons", "Hidden");
    assert_unchecked_child(&menu, "Close buttons", "Always visible");
    assert_unchecked_child(&menu, "Close buttons", "On hover (Alt-Tab / Win-Tab)");
    move_pointer(tile_center_rect(bounds));
    assert!(
        wait_until(TIMEOUT, || close_button_surface_is_hidden(bounds)),
        "the tray-hidden close button remained in the active real session"
    );

    assert!(application.select_tray_menu_item("Close buttons", "Always visible"));
    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be reopened after restoring close buttons");
    assert_checked_child(&menu, "Close buttons", "Always visible");
    assert_unchecked_child(&menu, "Close buttons", "On hover (Alt-Tab / Win-Tab)");
    assert_unchecked_child(&menu, "Close buttons", "Hidden");
    move_pointer(ScreenPoint::new(
        application.bounds().left + 1,
        application.bounds().top + 1,
    ));
    assert!(
        wait_until(TIMEOUT, || close_button_is_visible_in(bounds)),
        "the always-visible black-backed close button was not restored live"
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

fn borderless_fixture(title: &str, color: u32, bounds: RECT) -> FixtureWindow {
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

fn tile_center(tile: LiveTile) -> ScreenPoint {
    tile_center_rect(tile.bounds)
}

fn tile_center_rect(bounds: RECT) -> ScreenPoint {
    ScreenPoint::new(
        bounds.left + (bounds.right - bounds.left) / 2,
        bounds.top + (bounds.bottom - bounds.top) / 2,
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
    close_button_pixel_counts(bounds)
        .is_some_and(|(dark_pixels, white_pixels)| dark_pixels >= 100 && white_pixels >= 8)
}

fn close_button_surface_is_hidden(bounds: RECT) -> bool {
    close_button_pixel_counts(bounds)
        .is_some_and(|(dark_pixels, white_pixels)| dark_pixels < 50 && white_pixels < 8)
}

fn windows_close_button_is_visible(bounds: RECT) -> bool {
    close_button_pixel_counts(bounds)
        .is_some_and(|(dark_pixels, white_pixels)| white_pixels >= 8 && dark_pixels < 50)
}

fn close_button_pixel_counts(bounds: RECT) -> Option<(usize, usize)> {
    let image = capture_screen_image(bounds)?;
    let area = close_button_rect(bounds);
    Some((
        count_pixels(&image, area, is_black),
        count_pixels(&image, area, is_white),
    ))
}

fn tile_has_color(bounds: RECT, expected: u32) -> bool {
    unsafe {
        let _ = DwmFlush();
    }
    let x = bounds.left + (bounds.right - bounds.left) * 3 / 4;
    let y = bounds.top + (bounds.bottom - bounds.top) * 3 / 4;
    screen_pixel(x, y).is_some_and(|actual| color_distance(actual, expected) <= 55)
}

fn move_pointer(point: ScreenPoint) {
    assert!(CursorPosition::move_to(point.x, point.y));
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

fn assert_checked_child(menu: &[frigotab_acceptance::TrayMenuItem], parent: &str, child: &str) {
    let item = menu
        .iter()
        .find(|item| item.label == parent)
        .and_then(|item| item.children.iter().find(|item| item.label == child))
        .unwrap_or_else(|| panic!("tray item {parent} / {child} was not present"));
    assert!(item.checked, "tray item {parent} / {child} was not checked");
}

fn assert_unchecked_child(menu: &[frigotab_acceptance::TrayMenuItem], parent: &str, child: &str) {
    let item = menu
        .iter()
        .find(|item| item.label == parent)
        .and_then(|item| item.children.iter().find(|item| item.label == child))
        .unwrap_or_else(|| panic!("tray item {parent} / {child} was not present"));
    assert!(
        !item.checked,
        "tray item {parent} / {child} was unexpectedly checked"
    );
}

fn assert_fixture_tiles_match_current_layout(session: &LiveSession) {
    let layout = Layout::new(&WindowFinder::new().windows);
    for tile in fixture_tiles(session) {
        let expected = layout
            .bounds
            .get(&WindowHandle::new(tile.source))
            .unwrap_or_else(|| {
                panic!("surviving fixture {:?} is missing from layout", tile.source)
            });
        assert_eq!(expected.x, tile.bounds.left);
        assert_eq!(expected.y, tile.bounds.top);
        assert_eq!(expected.right(), tile.bounds.right);
        assert_eq!(expected.bottom(), tile.bounds.bottom);
    }
}
