#![cfg(windows)]

use std::time::{Duration, Instant};

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
const VK_A: u16 = 0x41;

fn fixtures() -> (FixtureWindow, FixtureWindow) {
    let first = FixtureWindow::show_with_options(
        "FrigoTab rapid-input first fixture",
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
        "FrigoTab rapid-input second fixture",
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
        "the first real foreground fixture received a keyboard message"
    );
    assert_eq!(
        0,
        second.keyboard_message_count(),
        "the second real foreground fixture received a keyboard message"
    );
}

fn wait_for_native_idle(
    application: &RunningFrigoTab,
    first: &FixtureWindow,
    foreground: &FixtureWindow,
) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        foreground.clear_keyboard_messages();
        assert!(application.send_marked_test_key(VK_A, false));
        assert!(application.send_marked_test_key(VK_A, true));
        if wait_until(Duration::from_millis(100), || {
            foreground.keyboard_message_count() >= 2
        }) {
            first.clear_keyboard_messages();
            foreground.clear_keyboard_messages();
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the real hook did not return to native idle behavior"
        );
    }
}

#[test]
fn rapid_marked_tab_release_keeps_the_real_sticky_session_and_focus() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let (first, second) = fixtures();
    let application = start_marked_application();
    first.clear_keyboard_messages();
    second.clear_keyboard_messages();

    // These transitions cross the production WH_KEYBOARD_LL callback. The
    // release is deliberately sent immediately after Tab-down, before the
    // UI has necessarily finished constructing the previews.
    assert!(application.send_marked_test_key(VK_LEFT_ALT, false));
    assert!(application.send_marked_test_key(VK_TAB, false));
    assert!(application.send_marked_test_key(VK_TAB, true));
    assert!(
        application.wait_visible(TIMEOUT),
        "the real switcher did not become visible after marked Alt+Tab"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == application.owner()),
        "the real switcher did not finish taking foreground focus"
    );

    assert!(
        application.is_visible(),
        "an immediate marked Tab release closed the sticky session"
    );
    assert_eq!(
        application.owner(),
        foreground_window(),
        "an immediate marked Tab release changed foreground focus"
    );

    assert!(application.send_marked_test_key(VK_LEFT_ALT, true));
    assert!(
        application.is_visible(),
        "releasing marked Alt changed the default sticky behavior"
    );
    assert_eq!(
        application.owner(),
        foreground_window(),
        "releasing marked Alt moved focus away from the sticky session"
    );

    // Close through the same marked hook so the test leaves no session or
    // pending release behind for the next acceptance process.
    assert!(application.send_marked_test_key(VK_ESCAPE, false));
    assert!(application.wait_hidden(TIMEOUT));
    assert!(application.send_marked_test_key(VK_ESCAPE, true));
    assert_no_fixture_keyboard_messages(&first, &second);
}

#[test]
fn rapid_marked_key_burst_recovers_for_a_second_gesture_without_leaking_releases() {
    let _serial = serial_guard();
    set_per_monitor_dpi_awareness();
    let (first, second) = fixtures();
    let application = start_marked_application();
    first.clear_keyboard_messages();
    second.clear_keyboard_messages();

    // Queue a complete gesture without waiting between transitions. This is
    // the observable acceptance equivalent of a busy/admission boundary: the
    // hook must retain the physical key lifetime even while UI callbacks are
    // still being drained, and a later gesture must not inherit its ledger.
    assert!(application.send_marked_test_key(VK_LEFT_ALT, false));
    assert!(application.send_marked_test_key(VK_TAB, false));
    assert!(application.send_marked_test_key(VK_TAB, true));
    assert!(application.send_marked_test_key(VK_LEFT_ALT, true));
    assert!(
        application.wait_visible(TIMEOUT),
        "the rapid marked gesture was not admitted by the real hook"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == application.owner()),
        "the rapid marked gesture did not finish taking foreground"
    );

    // Escape ends the first session. A fresh Alt+Tab must be admitted after
    // that close, proving that the previous matching releases did not poison
    // the next admission. It also exercises the critical close callback in a
    // queue that may still contain the burst above.
    assert!(application.send_marked_test_key(VK_ESCAPE, false));
    assert!(application.wait_hidden(TIMEOUT));
    assert!(application.send_marked_test_key(VK_ESCAPE, true));
    assert_no_fixture_keyboard_messages(&first, &second);

    // Distinct physical gestures cannot overlap in the same machine
    // instruction. Restore a known real foreground target and wait until that
    // handoff is observable before beginning the second chord.
    assert!(WindowHandle::new(second.handle()).set_foreground());
    assert!(wait_until(TIMEOUT, || foreground_window() == second.handle()));
    wait_for_native_idle(&application, &first, &second);

    assert!(application.send_marked_test_key(VK_LEFT_ALT, false));
    assert!(application.send_marked_test_key(VK_TAB, false));
    assert!(
        application.wait_visible(TIMEOUT),
        "the second marked Alt+Tab was not admitted after the rapid first gesture"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == application.owner()),
        "the second marked gesture did not finish taking foreground focus"
    );
    assert!(application.send_marked_test_key(VK_TAB, true));
    assert!(application.send_marked_test_key(VK_LEFT_ALT, true));
    assert!(application.send_marked_test_key(VK_ESCAPE, false));
    assert!(application.wait_hidden(TIMEOUT));
    assert!(application.send_marked_test_key(VK_ESCAPE, true));

    // Neither the first matching release nor the second one may escape to
    // the old target. The fixtures are real HWNDs, not test doubles.
    assert_no_fixture_keyboard_messages(&first, &second);
}
