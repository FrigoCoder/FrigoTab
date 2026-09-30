#![cfg(windows)]

use std::time::Duration;

use frigotab::input::MARKED_TEST_INPUT_EXTRA_INFO;
use frigotab::window::WindowHandle;
use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, GREEN, MAGENTA, RunningFrigoTab, TestKeyInput,
    foreground_window, serial_guard, set_per_monitor_dpi_awareness, wait_until,
};
use windows_sys::Win32::Foundation::RECT;

const TIMEOUT: Duration = Duration::from_secs(5);
const VK_LEFT_ALT_KEY: u16 = 0xa4;
const VK_TAB_KEY: u16 = 0x09;
const VK_2_KEY: u16 = 0x32;
const VK_ESCAPE_KEY: u16 = 0x1b;

fn marked_key(virtual_key: u16, key_up: bool) -> TestKeyInput {
    TestKeyInput::marked(virtual_key, key_up, MARKED_TEST_INPUT_EXTRA_INFO)
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
fn marked_hook_reopens_after_sticky_digit_activation_before_alt_release() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let first_fixture = FixtureWindow::show_with_options(
        "FrigoTab marked-input first fixture",
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
        "FrigoTab marked-input second fixture",
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

    // Digit selection activates a real candidate and closes Sticky mode while
    // marked Alt remains held. Balance the digit before the second cycle.
    assert!(application.send_marked_test_key(VK_2_KEY, false));
    assert!(
        application.wait_hidden(TIMEOUT),
        "Sticky digit selection did not activate and close the session"
    );
    assert!(application.send_marked_test_key(VK_2_KEY, true));
    // Preserve a short human key-transition interval between committing one
    // selection and pressing Tab again while the same Alt remains held.
    std::thread::sleep(Duration::from_millis(50));

    assert!(application.send_marked_test_key(VK_TAB_KEY, false));
    assert!(
        application.wait_visible(TIMEOUT),
        "Tab did not reopen the session while marked Alt remained held"
    );
    assert!(application.send_marked_test_key(VK_TAB_KEY, true));
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, true));

    assert!(application.send_marked_test_key(VK_ESCAPE_KEY, false));
    assert!(
        application.wait_hidden(TIMEOUT),
        "Escape did not close the reopened real session"
    );
    assert!(application.send_marked_test_key(VK_ESCAPE_KEY, true));

    assert!(
        wait_until(TIMEOUT, || {
            let foreground = foreground_window();
            !foreground.is_null()
                && foreground != application.owner()
                && WindowHandle::new(foreground).is_valid()
        }),
        "the final foreground window was not a real native target"
    );
    assert_no_keyboard_messages(&first_fixture, "first fixture");
    assert_no_keyboard_messages(&second_fixture, "second fixture");
}
