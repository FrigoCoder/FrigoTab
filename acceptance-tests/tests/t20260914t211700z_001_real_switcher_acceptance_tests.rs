#![cfg(windows)]

//! Black-box acceptance coverage for the real Alt-Tab replacement.
//!
//! Every scenario below drives the launched executable through the same
//! marked `SendInput` path used by the low-level hook.  The assertions observe
//! only process/window state and messages delivered to real fixture windows;
//! no controller, session, or selection implementation is constructed here.

use std::thread;
use std::time::{Duration, Instant};

use frigotab::geometry::ScreenPoint;
use frigotab::window::WindowHandle;
use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, GREEN, MAGENTA, ORANGE, RunningFrigoTab, capture_screen_image,
    color_distance, foreground_window, is_layered, is_window_visible, pump_messages, serial_guard,
    set_per_monitor_dpi_awareness, visible_owned_layered_windows, wait_until, window_bounds,
    window_owner,
};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{SetCursorPos, WS_EX_APPWINDOW, WS_POPUP};

const TIMEOUT: Duration = Duration::from_secs(5);

const VK_LEFT_ALT: u16 = 0xa4;
const VK_LEFT_SHIFT: u16 = 0xa0;
const VK_TAB: u16 = 0x09;
const VK_ESCAPE: u16 = 0x1b;
const VK_F4: u16 = 0x73;
const VK_1: u16 = 0x31;
const VK_A: u16 = 0x41;
const VK_NUMPAD2: u16 = 0x62;

struct TestWorld {
    application: RunningFrigoTab,
    fixtures: Vec<FixtureWindow>,
}

impl TestWorld {
    fn marked() -> Self {
        Self::new(true)
    }

    fn normal() -> Self {
        Self::new(false)
    }

    fn new(accept_marked_input: bool) -> Self {
        set_per_monitor_dpi_awareness();
        let fixtures = create_fixtures();
        let application = if accept_marked_input {
            RunningFrigoTab::start_accepting_marked_test_input()
                .expect("FrigoTab did not start in marked-input acceptance mode")
        } else {
            RunningFrigoTab::start().expect("FrigoTab did not start")
        };

        let initial = fixtures
            .last()
            .expect("the black-box world requires a foreground fixture");
        focus_fixture(initial);
        for fixture in &fixtures {
            fixture.clear_keyboard_messages();
        }

        Self {
            application,
            fixtures,
        }
    }

    fn send_key(&self, virtual_key: u16, key_up: bool) {
        assert!(
            self.application.send_marked_test_key(virtual_key, key_up),
            "marked keyboard transition was rejected (vk={virtual_key:#x}, up={key_up})"
        );
    }

    fn open_holding_alt(&self) {
        self.focus_initial();
        self.send_key(VK_LEFT_ALT, false);
        self.send_key(VK_TAB, false);
        assert!(
            self.application.wait_visible(TIMEOUT),
            "the marked Alt+Tab chord did not open the real session"
        );
        assert!(
            wait_until(TIMEOUT, || {
                visible_owned_layered_windows(self.application.owner()).len() >= self.fixtures.len()
            }),
            "the real session did not expose one preview overlay per fixture"
        );
        self.send_key(VK_TAB, true);
        assert!(
            wait_until(TIMEOUT, || foreground_window() == self.application.owner()),
            "the real session owner did not become foreground"
        );
    }

    fn open_without_releasing_tab(&self) {
        self.focus_initial();
        self.send_key(VK_LEFT_ALT, false);
        self.send_key(VK_TAB, false);
        assert!(
            self.application.wait_visible(TIMEOUT),
            "the marked Alt+Tab chord did not open the real session"
        );
    }

    fn cancel(&self, alt_is_held: bool) {
        self.send_key(VK_ESCAPE, false);
        assert!(
            self.application.wait_hidden(TIMEOUT),
            "Escape did not close the real session"
        );
        self.send_key(VK_ESCAPE, true);
        if alt_is_held {
            self.send_key(VK_LEFT_ALT, true);
        }
    }

    fn release_alt(&self) {
        self.send_key(VK_LEFT_ALT, true);
    }

    fn all_fixtures_are_alive(&self) -> bool {
        self.fixtures
            .iter()
            .all(|fixture| !fixture.handle().is_null() && is_window_visible(fixture.handle()))
    }

    fn focus_initial(&self) {
        let fixture = self
            .fixtures
            .last()
            .expect("the black-box world requires a foreground fixture");
        focus_fixture(fixture);

        // Visibility changes happen inside the UI callback slightly before
        // the hook publishes its idle state. Observe that boundary from the
        // outside: an ordinary key must make a complete round trip through
        // the real hook to the foreground fixture before a new gesture starts.
        let deadline = Instant::now() + TIMEOUT;
        loop {
            fixture.clear_keyboard_messages();
            self.send_key(VK_A, false);
            self.send_key(VK_A, true);
            if wait_until(Duration::from_millis(100), || {
                fixture.keyboard_message_count() >= 2
            }) {
                for fixture in &self.fixtures {
                    fixture.clear_keyboard_messages();
                }
                return;
            }
            assert!(
                Instant::now() < deadline,
                "the real keyboard hook did not return to native idle behavior"
            );
        }
    }

    fn assert_no_fixture_keyboard_messages(&self) {
        for fixture in &self.fixtures {
            assert!(
                !fixture.keyboard_message_log_overflowed(),
                "a real fixture keyboard log overflowed"
            );
            let messages = fixture.keyboard_messages();
            let leaked: Vec<_> = messages
                .into_iter()
                .filter(|message| {
                    matches!(
                        message.virtual_key,
                        VK_LEFT_ALT
                            | VK_LEFT_SHIFT
                            | VK_TAB
                            | VK_ESCAPE
                            | VK_F4
                            | VK_1
                            | VK_NUMPAD2
                    )
                })
                .collect();
            assert!(
                leaked.is_empty(),
                "a real fixture received keyboard messages from the intercepted gesture: {leaked:?}"
            );
        }
    }

    /// Select the orange fixture with the real pointer, traverse one complete
    /// keyboard cycle, and commit that same fixture in Tap mode.
    fn tap_cycle_fixture(&self, reverse: bool) -> HWND {
        self.focus_initial();
        self.send_key(VK_LEFT_ALT, false);
        self.send_key(VK_TAB, false);
        assert!(self.application.wait_visible(TIMEOUT));
        let overlays = wait_for_stable_overlays(&self.application, self.fixtures.len());
        let count = overlays.len();
        assert!(count >= self.fixtures.len());
        assert!(
            wait_until(TIMEOUT, || foreground_window() == self.application.owner()),
            "the real session did not finish taking foreground"
        );
        self.send_key(VK_TAB, true);

        let fixture_overlay = wait_for_fixture_overlay(&self.application, ORANGE);
        let fixture_bounds = window_bounds(fixture_overlay).expect("orange preview bounds");
        move_cursor(center(fixture_bounds));
        assert!(
            wait_until(TIMEOUT, || {
                overlay_color_samples(fixture_overlay, selected_overlay_color(ORANGE)) >= 500
            }),
            "the pointer did not select the real orange preview"
        );

        if reverse {
            self.send_key(VK_LEFT_SHIFT, false);
        }
        self.send_key(VK_TAB, false);
        self.send_key(VK_TAB, true);
        assert!(
            wait_until(TIMEOUT, || {
                overlay_color_samples(fixture_overlay, ORANGE) >= 500
            }),
            "one Tab did not move selection away from the orange preview"
        );
        for _ in 1..count {
            self.send_key(VK_TAB, false);
            self.send_key(VK_TAB, true);
        }
        assert!(
            wait_until(TIMEOUT, || {
                overlay_color_samples(fixture_overlay, selected_overlay_color(ORANGE)) >= 500
            }),
            "a complete Tab cycle did not wrap to the orange preview"
        );
        if reverse {
            self.send_key(VK_LEFT_SHIFT, true);
        }
        self.send_key(VK_LEFT_ALT, true);
        assert!(
            self.application.wait_hidden(TIMEOUT),
            "Tap-mode Alt release did not close the real session"
        );
        foreground_window()
    }
}

fn focus_fixture(fixture: &FixtureWindow) {
    assert!(
        WindowHandle::new(fixture.handle()).set_foreground(),
        "the fixture could not be made foreground"
    );
    assert!(
        wait_until(TIMEOUT, || foreground_window() == fixture.handle()),
        "the fixture did not become foreground"
    );
}

fn create_fixtures() -> Vec<FixtureWindow> {
    [
        ("FrigoTab black-box fixture green", GREEN),
        ("FrigoTab black-box fixture magenta", MAGENTA),
        ("FrigoTab black-box fixture orange", ORANGE),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (title, color))| {
        FixtureWindow::show_with_options(
            title,
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

fn wait_for_overlays(application: &RunningFrigoTab) -> Vec<HWND> {
    let mut overlays = Vec::new();
    assert!(wait_until(TIMEOUT, || {
        overlays = visible_owned_layered_windows(application.owner());
        !overlays.is_empty()
    }));
    overlays
}

fn wait_for_stable_overlays(application: &RunningFrigoTab, minimum: usize) -> Vec<HWND> {
    const STABLE_FOR: Duration = Duration::from_millis(100);

    let deadline = Instant::now() + TIMEOUT;
    let mut overlays = Vec::new();
    let mut stable_since = Instant::now();
    loop {
        pump_messages();
        let current = visible_owned_layered_windows(application.owner());
        if current != overlays {
            overlays = current;
            stable_since = Instant::now();
        }
        if overlays.len() >= minimum && stable_since.elapsed() >= STABLE_FOR {
            return overlays;
        }
        assert!(
            Instant::now() < deadline,
            "the real preview graph did not stabilize with at least {minimum} overlays"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn wait_for_fixture_overlay(application: &RunningFrigoTab, color: u32) -> HWND {
    let mut matching = None;
    assert!(
        wait_until(TIMEOUT, || {
            matching = visible_owned_layered_windows(application.owner())
                .into_iter()
                .map(|overlay| {
                    let samples = overlay_color_samples(overlay, color).max(overlay_color_samples(
                        overlay,
                        selected_overlay_color(color),
                    ));
                    (samples, overlay)
                })
                .max_by_key(|(samples, _)| *samples)
                .filter(|(samples, _)| *samples >= 500)
                .map(|(_, overlay)| overlay);
            matching.is_some()
        }),
        "the real preview for fixture color {color:#08x} did not become visible"
    );
    matching.expect("the matching preview disappeared after the wait")
}

fn overlay_color_samples(overlay: HWND, color: u32) -> usize {
    let Some(bounds) = window_bounds(overlay) else {
        return 0;
    };
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

fn wait_fixture_foreground(fixtures: &[FixtureWindow]) -> HWND {
    let mut foreground = foreground_window();
    assert!(
        wait_until(TIMEOUT, || {
            foreground = foreground_window();
            fixtures
                .iter()
                .any(|fixture| fixture.handle() == foreground && is_window_visible(foreground))
        }),
        "a real fixture did not become foreground"
    );
    foreground
}

fn assert_overlay_contract(application: &RunningFrigoTab) {
    let overlays = wait_for_overlays(application);
    for overlay in overlays {
        assert!(is_window_visible(overlay));
        assert_eq!(application.owner(), window_owner(overlay));
        assert!(is_layered(overlay));
    }
}

fn point_outside_overlays(overlays: &[HWND], owner_bounds: RECT) -> ScreenPoint {
    for y in (owner_bounds.top..owner_bounds.bottom).step_by(8) {
        for x in (owner_bounds.left..owner_bounds.right).step_by(8) {
            if overlays.iter().all(|&overlay| {
                window_bounds(overlay).is_none_or(|bounds| {
                    x < bounds.left || x >= bounds.right || y < bounds.top || y >= bounds.bottom
                })
            }) {
                return ScreenPoint::new(x, y);
            }
        }
    }
    panic!("no screen point outside the real preview HWNDs was available");
}

fn move_cursor(point: ScreenPoint) {
    assert!(unsafe { SetCursorPos(point.x, point.y) } != 0);
    pump_messages();
}

fn center(bounds: RECT) -> ScreenPoint {
    ScreenPoint::new(
        bounds.left + (bounds.right - bounds.left) / 2,
        bounds.top + (bounds.bottom - bounds.top) / 2,
    )
}

#[test]
fn alt_tab_opens_the_real_switcher_and_selects_a_real_preview() {
    let _serial = serial_guard();
    let world = TestWorld::marked();

    world.open_holding_alt();
    assert!(world.application.is_visible());
    assert_overlay_contract(&world.application);
    assert!(world.all_fixtures_are_alive());
    world.assert_no_fixture_keyboard_messages();

    world.release_alt();
    world.cancel(false);
}

#[test]
fn holding_alt_cycles_forward_and_wraps_through_real_previews() {
    let _serial = serial_guard();
    let world = TestWorld::marked();
    assert!(
        world
            .application
            .select_tray_menu_item("Alt-Tab behavior", "Tap (classic)")
    );

    let wrapped = world.tap_cycle_fixture(false);
    assert_eq!(
        world.fixtures.last().expect("orange fixture").handle(),
        wrapped,
        "forward Tab did not wrap at the last preview"
    );
    assert!(world.all_fixtures_are_alive());
    world.assert_no_fixture_keyboard_messages();
}

#[test]
fn shift_alt_tab_cycles_backward_and_wraps_through_real_previews() {
    let _serial = serial_guard();
    let world = TestWorld::marked();
    assert!(
        world
            .application
            .select_tray_menu_item("Alt-Tab behavior", "Tap (classic)")
    );

    let wrapped = world.tap_cycle_fixture(true);
    assert_eq!(
        world.fixtures.last().expect("orange fixture").handle(),
        wrapped,
        "reverse Tab did not wrap at the first preview"
    );
    assert!(world.all_fixtures_are_alive());
    world.assert_no_fixture_keyboard_messages();
}

#[test]
fn releasing_alt_leaves_the_real_switcher_open_for_deliberate_selection() {
    let _serial = serial_guard();
    let world = TestWorld::marked();

    world.open_holding_alt();
    world.release_alt();
    assert!(world.application.wait_visible(TIMEOUT));
    assert_eq!(world.application.owner(), foreground_window());
    assert!(world.all_fixtures_are_alive());
    world.assert_no_fixture_keyboard_messages();

    world.cancel(false);
}

#[test]
fn escape_cancels_the_real_session() {
    let _serial = serial_guard();
    let mut world = TestWorld::marked();

    world.open_holding_alt();
    world.cancel(true);
    assert!(!world.application.is_visible());
    assert!(
        !world
            .application
            .has_exited()
            .expect("could not query FrigoTab")
    );
    assert!(world.all_fixtures_are_alive());
    world.assert_no_fixture_keyboard_messages();
}

#[test]
fn alt_f4_cancels_the_real_session() {
    let _serial = serial_guard();
    let mut world = TestWorld::marked();

    world.open_holding_alt();
    world.send_key(VK_F4, false);
    assert!(world.application.wait_hidden(TIMEOUT));
    world.send_key(VK_F4, true);
    world.send_key(VK_LEFT_ALT, true);

    assert!(
        !world
            .application
            .has_exited()
            .expect("could not query FrigoTab")
    );
    assert!(world.all_fixtures_are_alive());
    world.assert_no_fixture_keyboard_messages();
}

#[test]
fn number_one_activates_the_first_real_preview() {
    let _serial = serial_guard();
    let world = TestWorld::marked();

    world.open_holding_alt();
    world.send_key(VK_1, false);
    assert!(world.application.wait_hidden(TIMEOUT));
    world.send_key(VK_1, true);
    world.send_key(VK_LEFT_ALT, true);

    let target = wait_fixture_foreground(&world.fixtures);
    assert!(
        world
            .fixtures
            .iter()
            .any(|fixture| fixture.handle() == target)
    );
    assert!(world.all_fixtures_are_alive());
    world.assert_no_fixture_keyboard_messages();
}

#[test]
fn number_pad_two_activates_the_second_real_preview() {
    let _serial = serial_guard();
    let world = TestWorld::marked();

    world.open_holding_alt();
    world.send_key(VK_1, false);
    assert!(world.application.wait_hidden(TIMEOUT));
    world.send_key(VK_1, true);
    world.send_key(VK_LEFT_ALT, true);
    let first = wait_fixture_foreground(&world.fixtures);

    world.open_holding_alt();
    world.send_key(VK_NUMPAD2, false);
    assert!(world.application.wait_hidden(TIMEOUT));
    world.send_key(VK_NUMPAD2, true);
    world.send_key(VK_LEFT_ALT, true);
    let second = wait_fixture_foreground(&world.fixtures);

    assert_ne!(first, second, "Numpad 2 activated the same preview as 1");
    assert!(world.all_fixtures_are_alive());
    world.assert_no_fixture_keyboard_messages();
}

#[test]
fn pointer_hover_and_click_select_and_activate_the_exact_real_preview() {
    let _serial = serial_guard();
    let world = TestWorld::marked();

    world.open_holding_alt();
    let overlay = wait_for_fixture_overlay(&world.application, ORANGE);
    let bounds = window_bounds(overlay).expect("the preview overlay had no bounds");
    let point = center(bounds);
    assert!(
        world.application.click_at(point.x, point.y),
        "the real pointer click was rejected"
    );

    assert!(world.application.wait_hidden(TIMEOUT));
    world.send_key(VK_LEFT_ALT, true);
    let target = wait_fixture_foreground(&world.fixtures);
    assert_eq!(
        world.fixtures.last().expect("orange fixture").handle(),
        target,
        "the pointer click did not activate the preview that was clicked"
    );
    assert!(visible_owned_layered_windows(world.application.owner()).is_empty());
    assert!(world.all_fixtures_are_alive());
}

#[test]
fn moving_outside_the_tiles_clears_selection_and_alt_tab_restores_it() {
    let _serial = serial_guard();
    let world = TestWorld::marked();

    world.open_holding_alt();
    let overlays = wait_for_overlays(&world.application);
    let outside = point_outside_overlays(&overlays, world.application.bounds());
    move_cursor(outside);
    assert!(world.application.is_visible());
    assert!(!wait_for_overlays(&world.application).is_empty());

    // Alt is still held. A fresh Tab must restore a keyboard selection after
    // the pointer has cleared the current one.
    world.send_key(VK_TAB, false);
    world.send_key(VK_TAB, true);
    world.send_key(VK_1, false);
    assert!(world.application.wait_hidden(TIMEOUT));
    world.send_key(VK_1, true);
    world.send_key(VK_LEFT_ALT, true);
    let foreground = wait_fixture_foreground(&world.fixtures);
    assert!(
        world
            .fixtures
            .iter()
            .any(|fixture| fixture.handle() == foreground)
    );
}

#[test]
fn a_closed_target_leaves_the_real_session_available_for_cancellation() {
    let _serial = serial_guard();
    let mut world = TestWorld::marked();

    world.open_holding_alt();
    let selected = world.fixtures.len() - 1;
    let closed_handle = world.fixtures[selected].handle();
    world.fixtures[selected].close();
    assert!(!is_window_visible(closed_handle));
    world.release_alt();

    assert!(world.application.wait_visible(TIMEOUT));
    assert!(
        !world
            .application
            .has_exited()
            .expect("could not query FrigoTab")
    );
    assert!(
        world
            .fixtures
            .iter()
            .filter(|fixture| !fixture.handle().is_null())
            .all(|fixture| is_window_visible(fixture.handle()))
    );
    world.cancel(false);
}

#[test]
fn closing_and_reopening_recreates_real_native_previews() {
    let _serial = serial_guard();
    let world = TestWorld::marked();

    world.open_holding_alt();
    let initial_count = wait_for_overlays(&world.application).len();
    world.cancel(true);
    assert!(visible_owned_layered_windows(world.application.owner()).is_empty());

    world.open_holding_alt();
    assert_eq!(initial_count, wait_for_overlays(&world.application).len());
    world.cancel(true);
    assert!(world.all_fixtures_are_alive());
}

#[test]
fn injected_alt_tab_passes_through_without_opening_real_windows() {
    let _serial = serial_guard();
    let mut world = TestWorld::normal();

    world.send_key(VK_LEFT_ALT, false);
    world.send_key(VK_TAB, false);
    world.send_key(VK_TAB, true);
    world.send_key(VK_LEFT_ALT, true);
    // Dismiss a native switcher if the operating system displayed one after
    // the marked event was passed through.
    world.send_key(VK_ESCAPE, false);
    world.send_key(VK_ESCAPE, true);
    thread::sleep(Duration::from_millis(100));

    assert!(!world.application.is_visible());
    assert!(visible_owned_layered_windows(world.application.owner()).is_empty());
    assert!(
        !world
            .application
            .has_exited()
            .expect("could not query FrigoTab")
    );
}

#[test]
fn consumed_key_ups_remain_balanced_after_the_real_session_closes() {
    let _serial = serial_guard();
    let mut world = TestWorld::marked();

    world.open_without_releasing_tab();
    world.send_key(VK_ESCAPE, false);
    assert!(world.application.wait_hidden(TIMEOUT));
    // These releases arrive only after the session has closed. They must not
    // leak into a fixture or reopen the switcher.
    world.send_key(VK_TAB, true);
    world.send_key(VK_ESCAPE, true);
    world.send_key(VK_LEFT_ALT, true);
    thread::sleep(Duration::from_millis(100));

    assert!(!world.application.is_visible());
    assert!(visible_owned_layered_windows(world.application.owner()).is_empty());
    assert!(
        !world
            .application
            .has_exited()
            .expect("could not query FrigoTab")
    );
    world.assert_no_fixture_keyboard_messages();
}

#[test]
fn repeated_open_and_close_leaves_no_preview_forms_behind() {
    let _serial = serial_guard();
    let mut world = TestWorld::marked();

    for _ in 0..5 {
        world.open_holding_alt();
        assert!(!wait_for_overlays(&world.application).is_empty());
        world.cancel(true);
        assert!(
            wait_until(TIMEOUT, || visible_owned_layered_windows(
                world.application.owner()
            )
            .is_empty()),
            "a real preview overlay remained after closing the session"
        );
        assert!(
            !world
                .application
                .has_exited()
                .expect("could not query FrigoTab")
        );
    }
    assert!(world.all_fixtures_are_alive());
}
