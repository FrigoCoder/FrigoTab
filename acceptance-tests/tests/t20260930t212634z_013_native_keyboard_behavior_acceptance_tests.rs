#![cfg(windows)]

//! Native keyboard-behavior acceptance checks.
//!
//! Every scenario in this family starts the real FrigoTab executable, sends
//! transitions through its real low-level keyboard hook, and observes the
//! messages delivered to a real foreground fixture HWND.  These tests do not
//! call the switcher controller directly.

use std::thread;
use std::time::{Duration, Instant};

use frigotab::input::MARKED_TEST_INPUT_EXTRA_INFO;
use frigotab::window::WindowHandle;
use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, GREEN, RunningFrigoTab, TestKeyInput, foreground_window,
    serial_guard, set_per_monitor_dpi_awareness, wait_until,
};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_A, VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_MENU, VK_SHIFT, VK_TAB,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WS_POPUP,
};

const TIMEOUT: Duration = Duration::from_secs(5);
const WRONG_MARKER: usize = MARKED_TEST_INPUT_EXTRA_INFO ^ 1;

fn fixture() -> FixtureWindow {
    set_per_monitor_dpi_awareness();
    FixtureWindow::show_with_options(
        "FrigoTab native keyboard acceptance fixture",
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
    )
}

fn marked(virtual_key: u16, key_up: bool) -> TestKeyInput {
    TestKeyInput::marked(virtual_key, key_up, MARKED_TEST_INPUT_EXTRA_INFO)
}

fn send(application: &RunningFrigoTab, input: TestKeyInput) {
    assert!(
        application.send_test_key(input),
        "SendInput did not accept the test keyboard transition: {input:?}"
    );
}

fn foreground_fixture(fixture: &FixtureWindow) {
    assert!(
        WindowHandle::new(fixture.handle()).set_foreground(),
        "the fixture could not be made foreground before sending input"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == fixture.handle()),
        "the fixture did not become foreground before sending input"
    );
    fixture.clear_keyboard_messages();
}

fn wait_for_messages(fixture: &FixtureWindow, count: usize) {
    let mut observed = Vec::new();
    assert!(
        wait_until(TIMEOUT, || {
            observed = fixture.keyboard_messages();
            observed.len() >= count
        }),
        "the foreground fixture did not receive {count} keyboard messages; observed {observed:?}"
    );
}

fn assert_messages(fixture: &FixtureWindow, expected: &[(u32, u16, bool, bool)]) {
    wait_for_messages(fixture, expected.len());
    assert!(
        !fixture.keyboard_message_log_overflowed(),
        "the foreground fixture keyboard log overflowed"
    );
    let actual = fixture.keyboard_messages();
    assert_eq!(
        actual.len(),
        expected.len(),
        "unexpected keyboard-message count: {actual:?}"
    );
    for (index, (message, virtual_key, transition, previous_state)) in
        expected.iter().copied().enumerate()
    {
        let actual = actual[index];
        assert_eq!(
            actual.message, message,
            "unexpected Win32 keyboard message at position {index}: {actual:?}"
        );
        assert_eq!(
            actual.virtual_key, virtual_key,
            "unexpected virtual key at position {index}: {actual:?}"
        );
        assert_eq!(
            actual.repeat_count, 1,
            "unexpected repeat count at position {index}: {actual:?}"
        );
        assert_eq!(
            actual.transition, transition,
            "unexpected transition bit at position {index}: {actual:?}"
        );
        assert_eq!(
            actual.previous_state, previous_state,
            "unexpected previous-state bit at position {index}: {actual:?}"
        );
    }
}

fn assert_no_messages(fixture: &FixtureWindow) {
    let deadline = Instant::now() + Duration::from_millis(250);
    loop {
        let messages = fixture.keyboard_messages();
        assert!(
            messages.is_empty(),
            "the previous foreground fixture received keyboard input: {messages:?}"
        );
        if Instant::now() >= deadline {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn assert_hidden(application: &RunningFrigoTab) {
    assert!(
        application.wait_hidden(TIMEOUT),
        "the real FrigoTab session did not become hidden"
    );
    assert!(
        application.visible_owned_layered_windows().is_empty(),
        "a FrigoTab preview remained visible after the gesture ended"
    );
}

#[test]
fn bare_alt_is_replayed_as_one_balanced_native_system_key_pair() {
    let _serial = serial_guard();
    let fixture = fixture();
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    foreground_fixture(&fixture);

    send(&application, marked(VK_LMENU, false));
    send(&application, marked(VK_LMENU, true));

    assert_hidden(&application);
    assert_messages(
        &fixture,
        &[
            (WM_SYSKEYDOWN, VK_MENU, false, false),
            (WM_SYSKEYUP, VK_MENU, true, true),
        ],
    );
}

#[test]
fn alt_plus_ordinary_a_is_replayed_in_order_and_balanced() {
    let _serial = serial_guard();
    let fixture = fixture();
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    foreground_fixture(&fixture);

    send(&application, marked(VK_LMENU, false));
    send(&application, marked(VK_A, false));
    send(&application, marked(VK_A, true));
    send(&application, marked(VK_LMENU, true));

    assert_hidden(&application);
    assert_messages(
        &fixture,
        &[
            (WM_SYSKEYDOWN, VK_MENU, false, false),
            (WM_SYSKEYDOWN, VK_A, false, false),
            (WM_SYSKEYUP, VK_A, true, true),
            (WM_KEYUP, VK_MENU, true, true),
        ],
    );
}

#[test]
fn alt_plus_shift_without_tab_is_replayed_in_order_and_balanced() {
    let _serial = serial_guard();
    let fixture = fixture();
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    foreground_fixture(&fixture);

    send(&application, marked(VK_LMENU, false));
    send(&application, marked(VK_LSHIFT, false));
    send(&application, marked(VK_LSHIFT, true));
    send(&application, marked(VK_LMENU, true));

    assert_hidden(&application);
    assert_messages(
        &fixture,
        &[
            (WM_SYSKEYDOWN, VK_MENU, false, false),
            (WM_SYSKEYDOWN, VK_SHIFT, false, false),
            (WM_SYSKEYUP, VK_SHIFT, true, true),
            (WM_KEYUP, VK_MENU, true, true),
        ],
    );
}

#[test]
fn alt_first_reverse_switching_suppresses_the_entire_chord_from_the_old_target() {
    let _serial = serial_guard();
    let fixture = fixture();
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    foreground_fixture(&fixture);

    send(&application, marked(VK_LMENU, false));
    send(&application, marked(VK_LSHIFT, false));
    send(&application, marked(VK_TAB, false));
    assert!(
        application.wait_visible(TIMEOUT),
        "Alt-first reverse input did not open the real switcher"
    );
    send(&application, marked(VK_TAB, true));
    send(&application, marked(VK_LSHIFT, true));
    send(&application, marked(VK_LMENU, true));
    send(&application, marked(VK_ESCAPE, false));
    send(&application, marked(VK_ESCAPE, true));

    assert_hidden(&application);
    assert_no_messages(&fixture);
}

#[test]
fn shift_first_reverse_switching_balances_shift_before_the_switcher_takes_focus() {
    let _serial = serial_guard();
    let fixture = fixture();
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    foreground_fixture(&fixture);

    // Shift was already native before Alt began, so its down reaches the old
    // target.  The hook must replay the matching up before moving focus.
    send(&application, marked(VK_LSHIFT, false));
    wait_for_messages(&fixture, 1);
    send(&application, marked(VK_LMENU, false));
    send(&application, marked(VK_TAB, false));
    assert!(
        application.wait_visible(TIMEOUT),
        "Shift-first reverse input did not open the real switcher"
    );

    send(&application, marked(VK_TAB, true));
    send(&application, marked(VK_LSHIFT, true));
    send(&application, marked(VK_LMENU, true));
    send(&application, marked(VK_ESCAPE, false));
    send(&application, marked(VK_ESCAPE, true));

    assert_hidden(&application);
    assert_messages(
        &fixture,
        &[
            (WM_KEYDOWN, VK_SHIFT, false, false),
            (WM_KEYUP, VK_SHIFT, true, true),
        ],
    );
}

#[test]
fn ctrl_alt_plus_ordinary_a_stays_native_and_does_not_open_frigotab() {
    let _serial = serial_guard();
    let fixture = fixture();
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    foreground_fixture(&fixture);

    send(&application, marked(VK_LCONTROL, false));
    send(&application, marked(VK_LMENU, false));
    send(&application, marked(VK_A, false));
    send(&application, marked(VK_A, true));
    send(&application, marked(VK_LMENU, true));
    send(&application, marked(VK_LCONTROL, true));

    assert_hidden(&application);
    assert_messages(
        &fixture,
        &[
            (WM_KEYDOWN, VK_CONTROL, false, false),
            (WM_KEYDOWN, VK_MENU, false, false),
            (WM_KEYDOWN, VK_A, false, false),
            (WM_KEYUP, VK_A, true, true),
            (WM_SYSKEYUP, VK_MENU, true, true),
            (WM_KEYUP, VK_CONTROL, true, true),
        ],
    );
}

#[test]
fn unknown_a_is_quarantined_while_visible_and_a_later_native_a_still_arrives() {
    let _serial = serial_guard();
    let fixture = fixture();
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    foreground_fixture(&fixture);

    send(&application, marked(VK_LMENU, false));
    send(&application, marked(VK_TAB, false));
    assert!(
        application.wait_visible(TIMEOUT),
        "the real switcher did not become visible"
    );
    send(&application, marked(VK_TAB, true));

    send(&application, marked(VK_A, false));
    send(&application, marked(VK_A, true));
    assert_no_messages(&fixture);

    send(&application, marked(VK_ESCAPE, false));
    send(&application, marked(VK_ESCAPE, true));
    send(&application, marked(VK_LMENU, true));
    assert_hidden(&application);

    // The blocked release bit for A must be cleared by the matching up; a
    // subsequent native-looking A therefore reaches the old target normally.
    foreground_fixture(&fixture);
    // The close/visibility callback runs on the application UI thread. Allow
    // that callback to publish the idle state before testing the next native
    // transition through the still-running global hook.
    thread::sleep(Duration::from_millis(100));
    send(&application, marked(VK_A, false));
    send(&application, marked(VK_A, true));
    assert_messages(
        &fixture,
        &[
            (WM_KEYDOWN, VK_A, false, false),
            (WM_KEYUP, VK_A, true, true),
        ],
    );
}

#[test]
fn a_wrong_injection_marker_is_ignored_even_in_marked_acceptance_mode() {
    let _serial = serial_guard();
    let fixture = fixture();
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    foreground_fixture(&fixture);

    send(
        &application,
        TestKeyInput::with_options(VK_A, false, false, WRONG_MARKER),
    );
    send(
        &application,
        TestKeyInput::with_options(VK_A, false, true, WRONG_MARKER),
    );

    assert_hidden(&application);
    assert_messages(
        &fixture,
        &[
            (WM_KEYDOWN, VK_A, false, false),
            (WM_KEYUP, VK_A, true, true),
        ],
    );
}
