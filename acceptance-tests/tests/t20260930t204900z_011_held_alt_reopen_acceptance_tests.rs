#![cfg(windows)]

use std::time::Duration;

use frigotab::input::MARKED_TEST_INPUT_EXTRA_INFO;
use frigotab::window::WindowHandle;
use frigotab_acceptance::{
    CursorPosition, FixtureOptions, FixtureWindow, GREEN, MAGENTA, RunningFrigoTab, TestKeyInput,
    capture_screen_image, color_distance, foreground_window, serial_guard,
    set_per_monitor_dpi_awareness, visible_owned_layered_windows, wait_until, window_bounds,
};
use windows_sys::Win32::Foundation::RECT;

const TIMEOUT: Duration = Duration::from_secs(5);
const VK_LEFT_ALT_KEY: u16 = 0xa4;
const VK_TAB_KEY: u16 = 0x09;
const VK_ESCAPE_KEY: u16 = 0x1b;

fn marked_key(virtual_key: u16, key_up: bool) -> TestKeyInput {
    TestKeyInput::marked(virtual_key, key_up, MARKED_TEST_INPUT_EXTRA_INFO)
}

fn center(bounds: RECT) -> (i32, i32) {
    (
        bounds.left + (bounds.right - bounds.left) / 2,
        bounds.top + (bounds.bottom - bounds.top) / 2,
    )
}

fn wait_for_colored_tile(application: &RunningFrigoTab, color: u32) -> (i32, i32) {
    let mut tile = None;
    assert!(
        wait_until(TIMEOUT, || {
            tile = visible_owned_layered_windows(application.owner())
                .into_iter()
                .filter_map(|window| {
                    let bounds = window_bounds(window)?;
                    let image = capture_screen_image(bounds)?;
                    let mut samples = 0usize;
                    for y in (bounds.top..bounds.bottom).step_by(4) {
                        for x in (bounds.left..bounds.right).step_by(4) {
                            if image
                                .pixel(x, y)
                                .is_some_and(|pixel| color_distance(pixel, color) <= 55)
                            {
                                samples += 1;
                            }
                        }
                    }
                    Some((samples, center(bounds)))
                })
                .max_by_key(|(samples, _)| *samples)
                .filter(|(samples, _)| *samples >= 500)
                .map(|(_, center)| center);
            tile.is_some()
        }),
        "the real fixture preview did not become visible"
    );
    tile.expect("the matching fixture preview disappeared after the wait")
}

fn assert_no_keyboard_messages(fixture: &FixtureWindow, name: &str) {
    assert!(
        !fixture.keyboard_message_log_overflowed(),
        "{name} keyboard message log overflowed"
    );
    let messages = fixture.keyboard_messages();
    assert!(
        messages.is_empty(),
        "{name} received keyboard messages from the intercepted gesture: {messages:?}"
    );
}

#[test]
fn marked_hook_reopens_after_real_mouse_selection_while_alt_remains_held() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let first_fixture = FixtureWindow::show_with_options(
        "FrigoTab marked mouse first fixture",
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
    let second_fixture = FixtureWindow::show_with_options(
        "FrigoTab marked mouse second fixture",
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
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    first_fixture.clear_keyboard_messages();
    second_fixture.clear_keyboard_messages();

    assert!(
        WindowHandle::new(second_fixture.handle()).set_foreground(),
        "the second real fixture could not be foregrounded before the intercepted gesture"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == second_fixture.handle()),
        "the second real fixture was not foreground before the intercepted gesture"
    );

    // Drive the real low-level hook. Alt remains held after Tab-up and while
    // the pointer selects a tile.
    assert_eq!(
        2,
        application.send_test_key_sequence(&[
            marked_key(VK_LEFT_ALT_KEY, false),
            marked_key(VK_TAB_KEY, false),
        ])
    );
    assert!(
        application.wait_visible(TIMEOUT),
        "the marked Alt+Tab chord did not open the real session"
    );
    assert!(wait_until(TIMEOUT, || foreground_window() == application.owner()));
    assert!(application.send_marked_test_key(VK_TAB_KEY, true));

    let tile = wait_for_colored_tile(&application, MAGENTA);
    let _cursor = CursorPosition::capture();
    assert!(
        application.click_at(tile.0, tile.1),
        "the real mouse click could not be sent to the selected preview"
    );
    assert!(
        application.wait_hidden(TIMEOUT),
        "the real mouse selection did not activate a fixture and close Sticky mode"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == second_fixture.handle()),
        "mouse selection did not foreground the clicked real fixture application"
    );
    // A human cannot press Tab in the same machine instruction as the mouse
    // release. Preserve a short physical transition interval so this scenario
    // tests held-Alt re-entry, not an impossible cross-thread queue race.
    std::thread::sleep(Duration::from_millis(50));

    // The same physical Alt is still down. A fresh marked Tab must reopen the
    // real process session instead of being swallowed by the prior Alt ledger.
    assert!(application.send_marked_test_key(VK_TAB_KEY, false));
    assert!(
        application.wait_visible(TIMEOUT),
        "Tab did not reopen the real session while marked Alt remained held"
    );
    assert!(application.send_marked_test_key(VK_TAB_KEY, true));
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, true));

    // Sticky mode remains open after Alt release. Escape is sent through the
    // same marked hook path to close it and leave no pending key ledger.
    assert!(application.send_marked_test_key(VK_ESCAPE_KEY, false));
    assert!(
        application.wait_hidden(TIMEOUT),
        "Escape did not close the reopened real session"
    );
    assert!(application.send_marked_test_key(VK_ESCAPE_KEY, true));

    assert_no_keyboard_messages(&first_fixture, "first fixture");
    assert_no_keyboard_messages(&second_fixture, "second fixture");
}
