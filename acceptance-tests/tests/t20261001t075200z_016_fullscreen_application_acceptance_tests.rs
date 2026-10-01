#![cfg(windows)]

//! Black-box coverage for native behaviors exposed by fullscreen applications.
//!
//! The scenarios use the shipped executable, real Win32 source windows, the
//! production hook, the system cursor clip, and the owner's actual Windows
//! messages. They deliberately do not change the machine's display mode; a
//! true exclusive-mode transition remains in the manual release matrix.

use std::thread;
use std::time::{Duration, Instant};

use frigotab::geometry::{Layout, virtual_screen_bounds};
use frigotab::window::{WindowFinder, WindowHandle};
use frigotab_acceptance::{
    CursorPosition, FixtureOptions, FixtureWindow, GREEN, MAGENTA, RunningFrigoTab,
    foreground_window, serial_guard, wait_until, window_bounds,
};
use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ClipCursor, GetClipCursor, GetCursorPos, IsWindow, PostMessageW, SWP_NOACTIVATE,
    SWP_NOOWNERZORDER, SWP_NOZORDER, SetCursorPos, SetWindowPos, WM_ACTIVATEAPP, WM_DISPLAYCHANGE,
    WM_DWMCOMPOSITIONCHANGED, WS_EX_APPWINDOW, WS_EX_NOREDIRECTIONBITMAP, WS_POPUP,
};

const TIMEOUT: Duration = Duration::from_secs(5);
const VK_LEFT_ALT_KEY: u16 = 0xa4;
const VK_TAB_KEY: u16 = 0x09;
const VK_ESCAPE_KEY: u16 = 0x1b;
const CLOSE_BUTTON_INSET: i32 = 8;
const CLOSE_BUTTON_SIZE: i32 = 32;

#[test]
fn a_display_notification_burst_rebuilds_the_real_sticky_session_in_place() {
    let _serial = serial_guard();
    let fixtures = real_fixtures("FrigoTab fullscreen transition");
    let mut application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");

    open_session(&application);
    let initial_overlays = stable_overlay_count(&application, fixtures.len());
    let initial_overlay_windows = application.visible_owned_layered_windows();
    assert_eq!(initial_overlays, initial_overlay_windows.len());

    let desktop = virtual_screen_bounds();
    assert_ne!(
        unsafe {
            SetWindowPos(
                application.owner(),
                std::ptr::null_mut(),
                desktop.left + 40,
                desktop.top + 40,
                desktop.right - desktop.left - 80,
                desktop.bottom - desktop.top - 80,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOZORDER,
            )
        },
        0
    );
    assert!(!same_rect(desktop, application.bounds()));
    let width = desktop.right - desktop.left;
    let height = desktop.bottom - desktop.top;
    let display_size = ((height as u32 & 0xffff) << 16) | (width as u32 & 0xffff);
    for _ in 0..4 {
        assert_ne!(
            unsafe {
                PostMessageW(
                    application.owner(),
                    WM_DISPLAYCHANGE,
                    32,
                    display_size as isize,
                )
            },
            0
        );
    }
    assert_ne!(
        unsafe { PostMessageW(application.owner(), WM_DWMCOMPOSITIONCHANGED, 0, 0) },
        0
    );
    // Exclusive-mode transitions can leave a stale deactivation notification
    // queued after FrigoTab already owns the foreground.
    assert_ne!(
        unsafe { PostMessageW(application.owner(), WM_ACTIVATEAPP, 0, 0) },
        0
    );

    let rebuilt = wait_until(TIMEOUT, || {
        let overlays = application.visible_owned_layered_windows();
        application.is_visible()
            && same_rect(desktop, application.bounds())
            && application.owner() == foreground_window()
            && initial_overlays == overlays.len()
            && overlays
                .iter()
                .all(|window| !initial_overlay_windows.contains(window))
    });
    if !rebuilt {
        let overlays = application.visible_owned_layered_windows();
        panic!(
            "the settled notification burst did not restore the live preview graph: visible={}, bounds={:?}, desktop={:?}, foreground={}, owner={}, overlays={}/{}, retained_handles={}",
            application.is_visible(),
            rect_tuple(application.bounds()),
            rect_tuple(desktop),
            foreground_window() as usize,
            application.owner() as usize,
            overlays.len(),
            initial_overlays,
            overlays
                .iter()
                .filter(|window| initial_overlay_windows.contains(window))
                .count()
        );
    }
    assert!(
        application.is_visible(),
        "a display-mode notification closed the sticky session"
    );
    assert!(
        !application
            .has_exited()
            .expect("process state was unreadable"),
        "FrigoTab exited during a display-mode transition"
    );
    assert!(same_rect(desktop, application.bounds()));
    assert_eq!(application.owner(), foreground_window());
    assert_eq!(
        initial_overlays,
        application.visible_owned_layered_windows().len(),
        "the settled display transition lost or duplicated preview overlays"
    );

    close_session(&application);
}

#[test]
fn the_visible_switcher_releases_a_reapplied_system_cursor_clip() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    let _clip = CursorClip::capture();
    let fixtures = real_fixtures("FrigoTab fullscreen cursor clip");
    let source_clip = fixtures[1].bounds();
    assert_ne!(unsafe { ClipCursor(&source_clip) }, 0);
    assert!(same_rect(source_clip, cursor_clip()));

    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");
    open_session(&application);

    // Some games reapply their confinement from a render/input loop after
    // losing activation. FrigoTab must keep the visible session usable rather
    // than relying on one automatic release during foreground transfer.
    assert_ne!(unsafe { ClipCursor(&source_clip) }, 0);
    assert!(same_rect(source_clip, cursor_clip()));

    assert!(
        wait_until(TIMEOUT, || !same_rect(cursor_clip(), source_clip)),
        "the previous fullscreen application kept the system cursor confined"
    );
    let desktop = virtual_screen_bounds();
    let outside = point_outside(source_clip, desktop);
    assert_ne!(unsafe { SetCursorPos(outside.x, outside.y) }, 0);
    let mut actual = POINT::default();
    assert_ne!(unsafe { GetCursorPos(&mut actual) }, 0);
    assert_eq!((outside.x, outside.y), (actual.x, actual.y));

    close_session(&application);
}

#[test]
fn a_no_redirection_source_closes_through_the_native_system_command() {
    let _serial = serial_guard();
    let _cursor = CursorPosition::capture().expect("the pointer position should be readable");
    let survivor = FixtureWindow::show_with_options(
        "FrigoTab protected fullscreen survivor",
        MAGENTA,
        fixture_options(rect(80, 80, 720, 560), 0),
    );
    let target = FixtureWindow::show_with_options(
        "FrigoTab protected fullscreen target",
        GREEN,
        fixture_options(rect(170, 150, 810, 630), WS_EX_NOREDIRECTIONBITMAP),
    );
    let target_handle = WindowHandle::new(target.handle());
    let mut application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");

    open_session(&application);
    let layout = Layout::new(&WindowFinder::new().windows);
    let expected = *layout
        .bounds
        .get(&target_handle)
        .expect("the no-redirection source was not laid out");
    assert!(
        wait_until(TIMEOUT, || {
            application
                .visible_owned_layered_windows()
                .into_iter()
                .filter_map(window_bounds)
                .any(|bounds| {
                    bounds.left == expected.x
                        && bounds.top == expected.y
                        && bounds.right == expected.right()
                        && bounds.bottom == expected.bottom()
                })
        }),
        "the source without a DWM redirection surface had no usable fallback tile"
    );
    let initial_overlays = stable_overlay_count(&application, 2);
    assert!(application.click_at(
        expected.right() - CLOSE_BUTTON_INSET - CLOSE_BUTTON_SIZE / 2,
        expected.y + CLOSE_BUTTON_INSET + CLOSE_BUTTON_SIZE / 2,
    ));

    assert!(
        wait_until(TIMEOUT, || {
            !is_valid_window(target.handle())
                && application.visible_owned_layered_windows().len() < initial_overlays
        }),
        "closing the no-redirection source left its stale fallback tile"
    );
    assert_eq!(1, target.system_close_request_count());
    assert!(application.is_visible());
    assert!(
        !application
            .has_exited()
            .expect("process state was unreadable"),
        "closing a no-redirection source terminated FrigoTab"
    );
    assert!(is_valid_window(survivor.handle()));

    close_session(&application);
}

fn real_fixtures(prefix: &str) -> Vec<FixtureWindow> {
    let fixtures: Vec<_> = [MAGENTA, GREEN]
        .into_iter()
        .enumerate()
        .map(|(index, color)| {
            FixtureWindow::show_with_options(
                &format!("{prefix} {}", index + 1),
                color,
                fixture_options(
                    rect(
                        80 + index as i32 * 90,
                        80 + index as i32 * 70,
                        720 + index as i32 * 90,
                        560 + index as i32 * 70,
                    ),
                    0,
                ),
            )
        })
        .collect();
    assert!(
        wait_until(TIMEOUT, || {
            let windows = WindowFinder::new().windows;
            fixtures
                .iter()
                .all(|fixture| windows.contains(&WindowHandle::new(fixture.handle())))
        }),
        "the real fixture windows did not become Alt-Tab candidates"
    );
    fixtures
}

fn fixture_options(bounds: RECT, additional_ex_style: u32) -> FixtureOptions {
    FixtureOptions {
        bounds,
        style: WS_POPUP,
        ex_style: WS_EX_APPWINDOW | additional_ex_style,
        activate: true,
        ..FixtureOptions::default()
    }
}

fn open_session(application: &RunningFrigoTab) {
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, false));
    assert!(application.send_marked_test_key(VK_TAB_KEY, false));
    assert!(application.wait_visible(TIMEOUT));
    assert!(application.send_marked_test_key(VK_TAB_KEY, true));
    assert!(application.send_marked_test_key(VK_LEFT_ALT_KEY, true));
    assert!(wait_until(TIMEOUT, || foreground_window() == application.owner()));
}

fn close_session(application: &RunningFrigoTab) {
    assert!(application.send_marked_test_key(VK_ESCAPE_KEY, false));
    assert!(application.wait_hidden(TIMEOUT));
    assert!(application.send_marked_test_key(VK_ESCAPE_KEY, true));
}

fn stable_overlay_count(application: &RunningFrigoTab, minimum: usize) -> usize {
    let deadline = Instant::now() + TIMEOUT;
    let mut previous = 0;
    let mut stable_since = Instant::now();
    loop {
        let current = application.visible_owned_layered_windows().len();
        if current != previous {
            previous = current;
            stable_since = Instant::now();
        } else if current >= minimum && stable_since.elapsed() >= Duration::from_millis(200) {
            return current;
        }
        assert!(
            Instant::now() < deadline,
            "the real preview overlays did not settle"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn cursor_clip() -> RECT {
    let mut bounds = RECT::default();
    assert_ne!(unsafe { GetClipCursor(&mut bounds) }, 0);
    bounds
}

fn point_outside(inside: RECT, desktop: RECT) -> POINT {
    for point in [
        POINT {
            x: desktop.left + 2,
            y: desktop.top + 2,
        },
        POINT {
            x: desktop.right - 3,
            y: desktop.bottom - 3,
        },
    ] {
        if point.x < inside.left
            || point.x >= inside.right
            || point.y < inside.top
            || point.y >= inside.bottom
        {
            return point;
        }
    }
    panic!("the source clip unexpectedly covers the whole virtual desktop");
}

fn is_valid_window(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 }
}

const fn same_rect(left: RECT, right: RECT) -> bool {
    left.left == right.left
        && left.top == right.top
        && left.right == right.right
        && left.bottom == right.bottom
}

const fn rect_tuple(value: RECT) -> (i32, i32, i32, i32) {
    (value.left, value.top, value.right, value.bottom)
}

const fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

struct CursorClip(RECT);

impl CursorClip {
    fn capture() -> Self {
        Self(cursor_clip())
    }
}

impl Drop for CursorClip {
    fn drop(&mut self) {
        unsafe {
            ClipCursor(&self.0);
        }
    }
}
