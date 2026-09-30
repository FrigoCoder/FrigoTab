#![cfg(windows)]

//! Black-box acceptance coverage for the optional per-thumbnail close button.
//!
//! Every scenario starts the shipped executable and drives it through the real
//! keyboard hook, tray menu, layered preview windows, and desktop pointer. The
//! fixtures are ordinary top-level Win32 windows; no in-process
//! session/controller substitute is involved.

use std::mem::size_of;
use std::time::Duration;

use frigotab::geometry::{Layout, ScreenPoint};
use frigotab::window::{WindowFinder, WindowHandle};
use frigotab_acceptance::{
    CursorPosition, FixtureOptions, FixtureWindow, GREEN, MAGENTA, ORANGE, RunningFrigoTab,
    ScreenCapture, TrayMenuItem, capture_screen_image, color_distance, foreground_window,
    pump_messages, serial_guard, set_per_monitor_dpi_awareness, wait_until, window_bounds,
};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT, SendInput,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindow, WS_EX_APPWINDOW, WS_POPUP};

const TIMEOUT: Duration = Duration::from_secs(5);
const VK_LEFT_ALT_KEY: u16 = 0xa4;
const VK_TAB_KEY: u16 = 0x09;
const VK_ESCAPE_KEY: u16 = 0x1b;
const CLOSE_BUTTON_INSET: i32 = 8;
const CLOSE_BUTTON_SIZE: i32 = 32;

#[derive(Clone, Copy)]
struct Preview {
    popup: HWND,
    bounds: RECT,
}

#[test]
fn default_close_button_closes_exact_source_and_refreshes_the_live_layout() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    set_per_monitor_dpi_awareness();
    let fixtures = real_fixtures("FrigoTab default close-button");
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");

    open_marked_session(&application);
    let target = wait_for_preview(&application, GREEN);
    let _other = wait_for_preview(&application, MAGENTA);
    let initial_overlay_count = application.visible_owned_layered_windows().len();
    assert!(
        initial_overlay_count >= 2,
        "the real session has too few previews"
    );
    assert!(
        close_button_is_visible_in(target.bounds),
        "the default close button did not render its black-backed X"
    );

    click_screen(close_button_center(target.bounds));
    assert!(
        wait_until(TIMEOUT, || {
            !is_valid_window(fixtures[1].handle())
                && !is_valid_window(target.popup)
                && wait_for_preview_optional(&application, GREEN).is_none()
                && application.visible_owned_layered_windows().len() < initial_overlay_count
        }),
        "the close button did not close its source and publish a refreshed preview graph"
    );
    assert!(
        application.is_visible(),
        "closing a source must keep the real switcher session open"
    );
    assert_eq!(
        application.owner(),
        foreground_window(),
        "closing a source must not activate the closed application"
    );
    assert!(
        is_valid_window(fixtures[0].handle()),
        "closing one preview unexpectedly closed another source HWND"
    );
    assert_fixture_layout(&application, &fixtures);
    assert!(
        wait_for_preview_optional(&application, MAGENTA).is_some(),
        "a surviving preview disappeared during close refresh"
    );

    close_marked_session(&application);
}

#[test]
fn hidden_close_buttons_leave_the_same_point_as_normal_tile_activation() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    set_per_monitor_dpi_awareness();
    let fixtures = real_fixtures("FrigoTab hidden close-button");
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");

    assert!(application.select_tray_menu_item("Close buttons", "Hidden"));
    assert_close_mode(
        &application
            .tray_menu()
            .expect("the real tray popup could not be inspected after selecting Hidden"),
        "Hidden",
    );
    open_marked_session(&application);

    let target = wait_for_preview(&application, GREEN);
    move_pointer(close_button_center(target.bounds));
    click_screen(close_button_center(target.bounds));

    assert!(
        application.wait_hidden(TIMEOUT),
        "a click with close buttons hidden did not follow normal tile activation"
    );
    assert!(
        is_valid_window(fixtures[1].handle()),
        "the hidden close-button setting closed the source instead of activating it"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == fixtures[1].handle()),
        "normal tile activation did not foreground the selected source"
    );
}

#[test]
fn windows_style_button_is_a_transparent_white_x_only_on_the_hovered_thumbnail() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    set_per_monitor_dpi_awareness();
    let fixtures = real_fixtures("FrigoTab hover close-button");
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");

    assert!(application.select_tray_menu_item("Close buttons", "On hover (Alt-Tab / Win-Tab)"));
    assert_close_mode(
        &application
            .tray_menu()
            .expect("the real tray popup could not be inspected after selecting HoverOnly"),
        "On hover (Alt-Tab / Win-Tab)",
    );

    open_marked_session_holding_alt(&application);
    let target = wait_for_preview(&application, MAGENTA);
    let other = wait_for_preview(&application, GREEN);
    let third = wait_for_preview(&application, ORANGE);
    let outside = point_outside_previews(&application);
    move_pointer(outside);
    assert!(wait_until(TIMEOUT, || {
        close_button_surface_is_hidden(target.bounds)
            && close_button_surface_is_hidden(other.bounds)
            && close_button_surface_is_hidden(third.bounds)
    }));

    move_pointer(tile_center(target.bounds));
    assert!(
        wait_until(TIMEOUT, || windows_close_button_is_visible(target.bounds)),
        "hovering a thumbnail did not reveal its transparent white close X"
    );
    assert!(
        close_button_surface_is_hidden(other.bounds),
        "hovering one thumbnail exposed a close X on another thumbnail"
    );

    move_pointer(outside);
    assert!(
        wait_until(TIMEOUT, || close_button_surface_is_hidden(target.bounds)),
        "the Windows-style close X remained after the pointer left the thumbnail"
    );

    // Keyboard selection is independent from pointer hover. Keep the marked
    // Alt transition down so this is the same held-Alt Tab gesture a user
    // performs, rather than an in-process controller call.
    assert!(application.send_marked_test_key(VK_TAB_KEY, false));
    assert!(application.send_marked_test_key(VK_TAB_KEY, true));
    assert!(wait_until(TIMEOUT, || {
        close_button_surface_is_hidden(target.bounds)
            && close_button_surface_is_hidden(other.bounds)
            && close_button_surface_is_hidden(third.bounds)
    }));

    move_pointer(close_button_center(target.bounds));
    click_screen(close_button_center(target.bounds));
    assert!(
        wait_until(TIMEOUT, || {
            !is_valid_window(fixtures[0].handle())
                && !is_valid_window(target.popup)
                && wait_for_preview_optional(&application, MAGENTA).is_none()
        }),
        "the hovered close X did not close the exact source"
    );
    assert!(application.is_visible());

    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, true));
    close_marked_session(&application);
}

#[test]
fn close_button_modes_are_selectable_from_the_real_tray_and_redraw_a_live_session() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    set_per_monitor_dpi_awareness();
    let fixtures = real_fixtures("FrigoTab tray close-button");
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");

    let menu = application
        .tray_menu()
        .expect("the real tray popup could not be inspected");
    assert_close_mode(&menu, "Always visible");

    assert!(application.select_tray_menu_item("Close buttons", "On hover (Alt-Tab / Win-Tab)"));
    assert_close_mode(
        &application
            .tray_menu()
            .expect("the real tray popup could not be reopened after selecting HoverOnly"),
        "On hover (Alt-Tab / Win-Tab)",
    );

    open_marked_session(&application);
    let target = wait_for_preview(&application, MAGENTA);
    let outside = point_outside_previews(&application);
    move_pointer(outside);
    assert!(wait_until(TIMEOUT, || {
        close_button_surface_is_hidden(target.bounds)
    }));
    move_pointer(tile_center(target.bounds));
    assert!(wait_until(TIMEOUT, || {
        windows_close_button_is_visible(target.bounds)
    }));

    assert!(application.select_tray_menu_item("Close buttons", "Hidden"));
    assert_close_mode(
        &application
            .tray_menu()
            .expect("the real tray popup could not be reopened after selecting Hidden"),
        "Hidden",
    );
    move_pointer(point_outside_previews(&application));
    assert!(wait_until(TIMEOUT, || {
        close_button_surface_is_hidden(target.bounds)
    }));

    assert!(application.select_tray_menu_item("Close buttons", "Always visible"));
    assert_close_mode(
        &application
            .tray_menu()
            .expect("the real tray popup could not be reopened after restoring Always visible"),
        "Always visible",
    );
    move_pointer(outside);
    assert!(wait_until(TIMEOUT, || {
        close_button_is_visible_in(target.bounds)
    }));

    // Close the real session through the same marked Escape path used by the
    // other scenarios. The fixtures remain owned by this test process.
    close_marked_session(&application);
    assert!(is_valid_window(fixtures[0].handle()));
}

fn real_fixtures(prefix: &str) -> Vec<FixtureWindow> {
    [MAGENTA, GREEN, ORANGE]
        .into_iter()
        .enumerate()
        .map(|(index, color)| {
            FixtureWindow::show_with_options(
                &format!("{prefix} {}", index + 1),
                color,
                FixtureOptions {
                    bounds: RECT {
                        left: 80 + index as i32 * 90,
                        top: 80 + index as i32 * 70,
                        right: 720 + index as i32 * 90,
                        bottom: 560 + index as i32 * 70,
                    },
                    style: WS_POPUP,
                    ex_style: WS_EX_APPWINDOW,
                    activate: true,
                    ..FixtureOptions::default()
                },
            )
        })
        .collect()
}

fn open_marked_session(application: &RunningFrigoTab) {
    open_marked_session_holding_alt(application);
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, true));
}

fn open_marked_session_holding_alt(application: &RunningFrigoTab) {
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, false));
    assert!(application.send_marked_test_key(VK_TAB_KEY, false));
    assert!(
        application.wait_visible(TIMEOUT),
        "the marked Alt+Tab chord did not open the real session"
    );
    assert!(application.send_marked_test_key(VK_TAB_KEY, true));
    assert!(wait_until(TIMEOUT, || foreground_window() == application.owner()));
}

fn close_marked_session(application: &RunningFrigoTab) {
    assert!(application.send_marked_test_key(VK_ESCAPE_KEY, false));
    assert!(
        application.wait_hidden(TIMEOUT),
        "the marked Escape did not close the real session"
    );
    assert!(application.send_marked_test_key(VK_ESCAPE_KEY, true));
}

fn wait_for_preview(application: &RunningFrigoTab, color: u32) -> Preview {
    let mut preview = None;
    assert!(
        wait_until(TIMEOUT, || {
            preview = wait_for_preview_optional(application, color);
            preview.is_some()
        }),
        "the real preview for color {color:#08x} did not become visible; sampled scores: {:?}",
        preview_color_scores(application, color)
    );
    preview.expect("preview became unavailable after the wait")
}

fn wait_for_preview_optional(application: &RunningFrigoTab, color: u32) -> Option<Preview> {
    unsafe {
        let _ = DwmFlush();
    }
    application
        .visible_owned_layered_windows()
        .into_iter()
        .filter_map(|popup| {
            let bounds = window_bounds(popup)?;
            let image = capture_screen_image(bounds)?;
            let samples = preview_color_samples(&image, bounds, color);
            Some((samples, Preview { popup, bounds }))
        })
        .max_by_key(|(samples, _)| *samples)
        .filter(|(samples, _)| *samples >= 100)
        .map(|(_, preview)| preview)
}

fn preview_color_scores(application: &RunningFrigoTab, color: u32) -> Vec<(HWND, usize)> {
    application
        .visible_owned_layered_windows()
        .into_iter()
        .filter_map(|popup| {
            let bounds = window_bounds(popup)?;
            let image = capture_screen_image(bounds)?;
            Some((popup, preview_color_samples(&image, bounds, color)))
        })
        .collect()
}

fn preview_color_samples(image: &ScreenCapture, bounds: RECT, color: u32) -> usize {
    let selected_color = selected_overlay_color(color);
    (bounds.top..bounds.bottom)
        .step_by(4)
        .flat_map(|y| (bounds.left..bounds.right).step_by(4).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            image.pixel(x, y).is_some_and(|pixel| {
                color_distance(pixel, color) <= 55 || color_distance(pixel, selected_color) <= 55
            })
        })
        .count()
}

fn selected_overlay_color(color: u32) -> u32 {
    // ApplicationWindow's 50%-alpha blue selection layer is composed over
    // the DWM source. Accept both this observable color and the untinted source
    // so a fixture remains identifiable while keyboard- or pointer-selected.
    let red = (color & 0xff) / 2;
    let green = ((color >> 8) & 0xff) / 2;
    let blue = (((color >> 16) & 0xff) + 255) / 2;
    red | (green << 8) | (blue << 16)
}

fn assert_fixture_layout(application: &RunningFrigoTab, fixtures: &[FixtureWindow]) {
    let layout = Layout::new(&WindowFinder::new().windows);
    for (fixture, color) in fixtures.iter().zip([MAGENTA, GREEN, ORANGE]) {
        if !is_valid_window(fixture.handle()) {
            continue;
        }
        let expected = layout
            .bounds
            .get(&WindowHandle::new(fixture.handle()))
            .unwrap_or_else(|| {
                panic!(
                    "surviving fixture {:?} is missing from layout",
                    fixture.handle()
                )
            });
        let preview = wait_for_preview(application, color);
        assert_eq!(expected.x, preview.bounds.left);
        assert_eq!(expected.y, preview.bounds.top);
        assert_eq!(expected.right(), preview.bounds.right);
        assert_eq!(expected.bottom(), preview.bounds.bottom);
    }
}

fn close_button_center(bounds: RECT) -> ScreenPoint {
    ScreenPoint::new(
        bounds.right - CLOSE_BUTTON_INSET - CLOSE_BUTTON_SIZE / 2,
        bounds.top + CLOSE_BUTTON_INSET + CLOSE_BUTTON_SIZE / 2,
    )
}

fn tile_center(bounds: RECT) -> ScreenPoint {
    ScreenPoint::new(
        bounds.left + (bounds.right - bounds.left) / 2,
        bounds.top + (bounds.bottom - bounds.top) / 2,
    )
}

fn point_outside_previews(application: &RunningFrigoTab) -> ScreenPoint {
    let owner = application.bounds();
    let previews = application
        .visible_owned_layered_windows()
        .into_iter()
        .filter_map(window_bounds)
        .collect::<Vec<_>>();
    for (x, y) in [
        (owner.left + 2, owner.top + 2),
        (owner.right - 3, owner.top + 2),
        (owner.left + 2, owner.bottom - 3),
        (owner.right - 3, owner.bottom - 3),
    ] {
        if previews.iter().all(|bounds| {
            x < bounds.left || x >= bounds.right || y < bounds.top || y >= bounds.bottom
        }) {
            return ScreenPoint::new(x, y);
        }
    }
    ScreenPoint::new(owner.left + 1, owner.top + 1)
}

fn move_pointer(point: ScreenPoint) {
    assert!(CursorPosition::move_to(point.x, point.y));
    pump_messages();
}

fn click_screen(point: ScreenPoint) {
    move_pointer(point);
    let inputs = [
        mouse_input(MOUSEEVENTF_LEFTDOWN),
        mouse_input(MOUSEEVENTF_LEFTUP),
    ];
    assert_eq!(
        unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            )
        },
        inputs.len() as u32,
        "Windows did not accept the real pointer click"
    );
    pump_messages();
}

fn mouse_input(flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

fn close_button_rect(bounds: RECT) -> RECT {
    RECT {
        left: bounds.right - CLOSE_BUTTON_INSET - CLOSE_BUTTON_SIZE,
        top: bounds.top + CLOSE_BUTTON_INSET,
        right: bounds.right - CLOSE_BUTTON_INSET,
        bottom: bounds.top + CLOSE_BUTTON_INSET + CLOSE_BUTTON_SIZE,
    }
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

fn assert_close_mode(menu: &[TrayMenuItem], selected: &str) {
    let parent = menu
        .iter()
        .find(|item| item.label == "Close buttons")
        .unwrap_or_else(|| panic!("the Close buttons submenu was not present"));
    for label in ["Always visible", "On hover (Alt-Tab / Win-Tab)", "Hidden"] {
        let item = parent
            .children
            .iter()
            .find(|item| item.label == label)
            .unwrap_or_else(|| panic!("tray item Close buttons / {label} was not present"));
        assert_eq!(
            item.checked,
            label == selected,
            "tray item Close buttons / {label} had the wrong check state"
        );
    }
}
