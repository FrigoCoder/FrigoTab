#![cfg(windows)]

//! End-to-end Alt-Tab behavior checks.
//!
//! These scenarios intentionally cross the launched process boundary. The
//! acceptance executable opts into the private marked-input mode and the
//! driver sends the chord through `WH_KEYBOARD_LL`; the fixture windows are
//! ordinary foreground HWNDs, not substitutes for the application's session.

use std::thread;
use std::time::{Duration, Instant};

use frigotab::window::WindowHandle;
use frigotab_acceptance::{
    CursorPosition, FixtureOptions, FixtureWindow, GREEN, MAGENTA, RunningFrigoTab,
    capture_screen_image, color_distance, foreground_window, serial_guard,
    visible_owned_layered_windows, wait_until, window_bounds,
};
use windows_sys::Win32::Foundation::RECT;

const TIMEOUT: Duration = Duration::from_secs(5);

const VK_LEFT_ALT_KEY: u16 = 0xa4;
const VK_TAB_KEY: u16 = 0x09;
const VK_LEFT_SHIFT_KEY: u16 = 0xa0;
const VK_ESCAPE_KEY: u16 = 0x1b;

fn fixtures() -> (FixtureWindow, FixtureWindow) {
    let first = FixtureWindow::show_with_options(
        "FrigoTab behavior first fixture",
        GREEN,
        FixtureOptions {
            bounds: RECT {
                left: 80,
                top: 80,
                right: 720,
                bottom: 560,
            },
            activate: true,
            ..FixtureOptions::default()
        },
    );
    let second = FixtureWindow::show_with_options(
        "FrigoTab behavior second fixture",
        MAGENTA,
        FixtureOptions {
            bounds: RECT {
                left: 160,
                top: 140,
                right: 800,
                bottom: 620,
            },
            activate: true,
            ..FixtureOptions::default()
        },
    );
    (first, second)
}

fn start_application(first: &FixtureWindow, second: &FixtureWindow) -> RunningFrigoTab {
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");

    // The responsive-owner startup barrier proves that the hook and UI loop
    // are ready before the first transition is sent.
    focus_fixture(second);
    assert!(
        foreground_window() == second.handle(),
        "the second fixture could not be made the initial foreground target"
    );

    // Both fixtures must remain independently addressable. The behavior
    // checks identify their live previews by pixels rather than depending on
    // EnumWindows ordering, which other desktop activity can legitimately
    // change.
    assert_ne!(first.handle(), second.handle());
    first.clear_keyboard_messages();
    second.clear_keyboard_messages();
    application
}

fn focus_fixture(fixture: &FixtureWindow) {
    assert!(WindowHandle::new(fixture.handle()).set_foreground());
    assert!(wait_until(TIMEOUT, || foreground_window() == fixture.handle()));
}

fn open_marked_session(application: &RunningFrigoTab) {
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, false));
    assert!(application.send_marked_test_key(VK_TAB_KEY, false));
    assert!(
        application.wait_visible(TIMEOUT),
        "the marked Alt+Tab chord did not open the real session"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == application.owner()),
        "the real session did not finish taking foreground"
    );
    assert!(application.send_marked_test_key(VK_TAB_KEY, true));
}

fn assert_real_selection_activated(application: &RunningFrigoTab) {
    assert!(
        application.wait_hidden(TIMEOUT),
        "Tap did not close the real session after committing a selection"
    );
    assert!(
        wait_until(TIMEOUT, || {
            let foreground = foreground_window();
            !foreground.is_null()
                && foreground != application.owner()
                && WindowHandle::new(foreground).is_valid()
        }),
        "Tap did not activate a real native target"
    );
}

fn close_marked_session(application: &RunningFrigoTab) {
    assert!(application.send_marked_test_key(VK_ESCAPE_KEY, false));
    assert!(
        application.wait_hidden(TIMEOUT),
        "Escape did not close the real session"
    );
    assert!(application.send_marked_test_key(VK_ESCAPE_KEY, true));
}

fn select_tap_from_real_tray(application: &RunningFrigoTab, initial: &FixtureWindow) {
    assert!(
        application.select_tray_menu_item("Alt-Tab behavior", "Tap (classic)"),
        "the real tray menu did not select Tap (classic)"
    );
    // Opening and clicking a tray popup can temporarily activate the tray
    // process. Restore the actual foreground target before the chord so the
    // initial preview and any later activation are observable on real HWNDs.
    focus_fixture(initial);
}

fn assert_no_fixture_keyboard_messages(first: &FixtureWindow, second: &FixtureWindow) {
    assert_eq!(
        0,
        first.keyboard_message_count(),
        "the first foreground fixture received a keyboard message"
    );
    assert_eq!(
        0,
        second.keyboard_message_count(),
        "the second foreground fixture received a keyboard message"
    );
    assert!(!first.keyboard_message_log_overflowed());
    assert!(!second.keyboard_message_log_overflowed());
}

fn point_outside_overlays(application: &RunningFrigoTab) -> (i32, i32) {
    let owner = application.bounds();
    let overlays: Vec<RECT> = application
        .visible_owned_overlays()
        .into_iter()
        .filter_map(window_bounds)
        .collect();

    for y in (owner.top..owner.bottom).step_by(8) {
        for x in (owner.left..owner.right).step_by(8) {
            if overlays.iter().all(|bounds| {
                x < bounds.left || x >= bounds.right || y < bounds.top || y >= bounds.bottom
            }) {
                return (x, y);
            }
        }
    }
    panic!("no point outside the real preview HWNDs was available");
}

fn move_pointer_outside_overlays(application: &RunningFrigoTab) {
    let (x, y) = point_outside_overlays(application);
    assert!(CursorPosition::move_to(x, y));
    // Mouse messages are delivered on FrigoTab's UI thread. Give it a short
    // bounded interval to clear the pointer selection before Alt is released.
    thread::sleep(Duration::from_millis(100));
}

fn select_fixture_with_pointer(application: &RunningFrigoTab, color: u32) {
    let bounds = wait_for_colored_preview(application, color);
    let point = (
        bounds.left + (bounds.right - bounds.left) / 2,
        bounds.top + (bounds.bottom - bounds.top) / 2,
    );
    assert!(application.move_pointer_to(point.0, point.1));
    assert!(
        wait_until(TIMEOUT, || {
            preview_color_samples(bounds, selected_overlay_color(color)) >= 100
        }),
        "the pointer did not select the real fixture preview"
    );
}

fn select_fixture_with_keyboard(application: &RunningFrigoTab, color: u32, reverse: bool) {
    let candidate_count = stable_candidate_count(application);
    let bounds = wait_for_colored_preview(application, color);
    if reverse {
        assert!(application.send_marked_test_key(VK_LEFT_SHIFT_KEY, false));
    }
    let mut selected = false;
    for _ in 0..candidate_count {
        assert!(application.send_marked_test_key(VK_TAB_KEY, false));
        assert!(application.send_marked_test_key(VK_TAB_KEY, true));
        if wait_until(Duration::from_millis(500), || {
            preview_color_samples(bounds, selected_overlay_color(color)) >= 100
        }) {
            selected = true;
            break;
        }
    }
    if reverse {
        assert!(application.send_marked_test_key(VK_LEFT_SHIFT_KEY, true));
    }
    assert!(selected, "keyboard traversal never selected the fixture");
}

fn stable_candidate_count(application: &RunningFrigoTab) -> usize {
    const STABLE_FOR: Duration = Duration::from_millis(100);

    let deadline = Instant::now() + TIMEOUT;
    let mut overlays = Vec::new();
    let mut stable_since = Instant::now();
    loop {
        let current = application.visible_owned_layered_windows();
        if current != overlays {
            overlays = current;
            stable_since = Instant::now();
        }
        if overlays.len() >= 2 && stable_since.elapsed() >= STABLE_FOR {
            return overlays.len();
        }
        assert!(
            Instant::now() < deadline,
            "the real preview graph did not stabilize with both fixture previews"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn wait_for_colored_preview(application: &RunningFrigoTab, color: u32) -> RECT {
    let mut preview = None;
    assert!(
        wait_until(TIMEOUT, || {
            preview = visible_owned_layered_windows(application.owner())
                .into_iter()
                .filter_map(window_bounds)
                .map(|bounds| {
                    let samples = preview_color_samples(bounds, color)
                        .max(preview_color_samples(bounds, selected_overlay_color(color)));
                    (samples, bounds)
                })
                .max_by_key(|(samples, _)| *samples)
                .filter(|(samples, _)| *samples >= 100)
                .map(|(_, bounds)| bounds);
            preview.is_some()
        }),
        "the real fixture preview did not become visible"
    );
    preview.expect("the fixture preview disappeared after the wait")
}

fn preview_color_samples(bounds: RECT, color: u32) -> usize {
    let Some(image) = capture_screen_image(bounds) else {
        return 0;
    };
    (bounds.top..bounds.bottom)
        .step_by(4)
        .flat_map(|y| (bounds.left..bounds.right).step_by(4).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            image
                .pixel(x, y)
                .is_some_and(|pixel| color_distance(pixel, color) <= 55)
        })
        .count()
}

fn selected_overlay_color(color: u32) -> u32 {
    let red = (color & 0xff) / 2;
    let green = ((color >> 8) & 0xff) / 2;
    let blue = (((color >> 16) & 0xff) + 255) / 2;
    red | (green << 8) | (blue << 16)
}

#[test]
fn default_sticky_behavior_keeps_the_real_session_open_after_alt_release() {
    let _serial = serial_guard();
    let (first, second) = fixtures();
    let application = start_application(&first, &second);

    open_marked_session(&application);
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, true));
    assert!(
        application.wait_visible(TIMEOUT),
        "default Sticky behavior closed the real session on Alt release"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == application.owner()),
        "the visible switcher did not remain the foreground window"
    );
    assert_no_fixture_keyboard_messages(&first, &second);

    close_marked_session(&application);
}

#[test]
fn tap_behavior_activates_the_initial_real_preview_on_alt_release() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    let (first, second) = fixtures();
    let application = start_application(&first, &second);
    select_tap_from_real_tray(&application, &second);

    open_marked_session(&application);
    select_fixture_with_pointer(&application, MAGENTA);
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, true));
    assert_real_selection_activated(&application);
    assert!(wait_until(TIMEOUT, || foreground_window() == second.handle()));
    assert_no_fixture_keyboard_messages(&first, &second);
}

#[test]
fn tap_behavior_activates_the_forward_selection_on_alt_release() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    let (first, second) = fixtures();
    let application = start_application(&first, &second);
    select_tap_from_real_tray(&application, &second);

    open_marked_session(&application);
    select_fixture_with_pointer(&application, MAGENTA);
    select_fixture_with_keyboard(&application, GREEN, false);
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, true));
    assert_real_selection_activated(&application);
    assert!(wait_until(TIMEOUT, || foreground_window() == first.handle()));
    assert_no_fixture_keyboard_messages(&first, &second);
}

#[test]
fn tap_behavior_activates_the_reverse_selection_on_alt_release() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    let (first, second) = fixtures();
    let application = start_application(&first, &second);
    select_tap_from_real_tray(&application, &second);

    open_marked_session(&application);
    select_fixture_with_pointer(&application, MAGENTA);
    select_fixture_with_keyboard(&application, GREEN, true);
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, true));
    assert_real_selection_activated(&application);
    assert!(wait_until(TIMEOUT, || foreground_window() == first.handle()));
    assert_no_fixture_keyboard_messages(&first, &second);
}

#[test]
fn tap_release_without_a_selection_keeps_the_real_session_available() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    let (first, second) = fixtures();
    let application = start_application(&first, &second);
    select_tap_from_real_tray(&application, &second);

    open_marked_session(&application);
    move_pointer_outside_overlays(&application);
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, true));
    assert!(
        application.wait_visible(TIMEOUT),
        "Tap committed a selection after the pointer left every real preview"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == application.owner()),
        "the real session did not remain foreground after an unselected Tap release"
    );
    assert_no_fixture_keyboard_messages(&first, &second);

    close_marked_session(&application);
}
