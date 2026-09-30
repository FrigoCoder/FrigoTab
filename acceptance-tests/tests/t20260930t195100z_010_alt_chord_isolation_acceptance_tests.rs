#![cfg(windows)]

use std::time::Duration;

use frigotab::window::WindowHandle;
use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, GREEN, MAGENTA, RunningFrigoTab, foreground_window,
    serial_guard, set_per_monitor_dpi_awareness, wait_until,
};
use windows_sys::Win32::Foundation::RECT;

const TIMEOUT: Duration = Duration::from_secs(5);
const VK_LEFT_ALT: u16 = 0xa4;
const VK_TAB: u16 = 0x09;
const VK_ESCAPE: u16 = 0x1b;

fn fixtures() -> (FixtureWindow, FixtureWindow) {
    let first = FixtureWindow::show_with_options(
        "FrigoTab Alt-chord first fixture",
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
        "FrigoTab Alt-chord second fixture",
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

fn start_marked_application() -> RunningFrigoTab {
    RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode")
}

fn assert_no_fixture_keyboard_messages(first: &FixtureWindow, second: &FixtureWindow) {
    assert_eq!(
        0,
        first.keyboard_message_count(),
        "the first real target received a keyboard message from the intercepted chord"
    );
    assert_eq!(
        0,
        second.keyboard_message_count(),
        "the second real target received a keyboard message from the intercepted chord"
    );
}

fn focus_fixture(fixture: &FixtureWindow) {
    assert!(WindowHandle::new(fixture.handle()).set_foreground());
    assert!(wait_until(TIMEOUT, || foreground_window() == fixture.handle()));
}

#[test]
fn sticky_handles_the_complete_marked_alt_tab_chord_without_leaking_focus_or_releases() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let (first, second) = fixtures();
    let application = start_marked_application();
    focus_fixture(&second);
    first.clear_keyboard_messages();
    second.clear_keyboard_messages();

    // The complete gesture is delivered through the production
    // WH_KEYBOARD_LL hook, not through SwitcherApplication directly.
    assert!(application.send_marked_test_key(VK_LEFT_ALT, false));
    assert!(application.send_marked_test_key(VK_TAB, false));
    assert!(
        application.wait_visible(TIMEOUT),
        "the marked Alt+Tab chord did not open the real sticky session"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == application.owner()),
        "the real sticky session did not finish becoming foreground"
    );

    assert!(application.send_marked_test_key(VK_TAB, true));
    assert!(application.send_marked_test_key(VK_LEFT_ALT, true));
    assert!(
        application.is_visible(),
        "the complete marked chord changed Sticky mode into Tap behavior"
    );
    assert_eq!(
        application.owner(),
        foreground_window(),
        "releasing the complete marked chord moved focus to a fixture"
    );

    assert!(application.send_marked_test_key(VK_ESCAPE, false));
    assert!(application.wait_hidden(TIMEOUT));
    assert!(application.send_marked_test_key(VK_ESCAPE, true));
    assert_no_fixture_keyboard_messages(&first, &second);
}

#[test]
fn tap_handles_the_complete_marked_alt_tab_chord_and_activates_the_real_target_once() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let (first, second) = fixtures();
    let application = start_marked_application();

    // The behavior is selected through the real notification-area menu. The
    // test never changes the controller or session object directly.
    assert!(
        application.select_tray_menu_item("Alt-Tab behavior", "Tap (classic)"),
        "the real tray menu did not select Tap (classic)"
    );
    focus_fixture(&second);
    first.clear_keyboard_messages();
    second.clear_keyboard_messages();

    assert!(application.send_marked_test_key(VK_LEFT_ALT, false));
    assert!(application.send_marked_test_key(VK_TAB, false));
    assert!(
        application.wait_visible(TIMEOUT),
        "the marked Alt+Tab chord did not open the real Tap session"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == application.owner()),
        "the real Tap session did not finish becoming foreground"
    );
    assert!(application.send_marked_test_key(VK_TAB, true));
    assert!(application.send_marked_test_key(VK_LEFT_ALT, true));
    assert!(
        application.wait_hidden(TIMEOUT),
        "Tap did not close the real session on marked Alt release"
    );
    assert!(wait_until(TIMEOUT, || {
        let foreground = foreground_window();
        !foreground.is_null()
            && foreground != application.owner()
            && WindowHandle::new(foreground).is_valid()
    }));
    assert_no_fixture_keyboard_messages(&first, &second);
}
