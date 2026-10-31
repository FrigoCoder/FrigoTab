#![cfg(windows)]

//! Black-box coverage for a fullscreen renderer which discards its live DWM
//! surface when FrigoTab takes foreground. The application HWND remains valid
//! and must retain both its preview and its numbered activation behavior.

use std::thread;
use std::time::{Duration, Instant};

use frigotab::geometry::{Layout, virtual_screen_bounds};
use frigotab::window::{WindowFinder, WindowHandle};
use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, GREEN, MAGENTA, RunningFrigoTab, color_distance,
    foreground_window, screen_pixel, serial_guard, wait_until, window_bounds,
};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::WindowsAndMessaging::{WS_EX_APPWINDOW, WS_EX_TOPMOST, WS_POPUP};

const TIMEOUT: Duration = Duration::from_secs(5);
const VK_LEFT_ALT: u16 = 0xa4;
const VK_TAB: u16 = 0x09;

#[test]
fn a_collapsed_fullscreen_surface_keeps_its_preview_and_numbered_activation() {
    let _serial = serial_guard();
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab.exe should start in marked-input acceptance mode");
    let desktop = virtual_screen_bounds();
    let _survivor = FixtureWindow::show_with_options(
        "FrigoTab fullscreen thumbnail survivor",
        MAGENTA,
        FixtureOptions {
            activate: false,
            ..FixtureOptions::default()
        },
    );
    let source = FixtureWindow::show_with_options(
        "FrigoTab collapsing fullscreen source",
        GREEN,
        FixtureOptions {
            bounds: desktop,
            style: WS_POPUP,
            ex_style: WS_EX_APPWINDOW | WS_EX_TOPMOST,
            activate: true,
            collapse_on_deactivate: true,
            ..FixtureOptions::default()
        },
    );
    let source_handle = WindowHandle::new(source.handle());
    assert!(source_handle.set_foreground());
    assert!(wait_until(TIMEOUT, || {
        source.handle() == foreground_window()
    }));

    let finder = WindowFinder::new();
    let source_index = finder
        .windows
        .iter()
        .position(|window| *window == source_handle)
        .expect("the fullscreen source should be an Alt-Tab candidate");
    assert!(
        source_index < 9,
        "the real source needs a numbered shortcut for this acceptance scenario"
    );
    let tile = *Layout::new(&finder.windows)
        .bounds
        .get(&source_handle)
        .expect("the fullscreen source should have production layout bounds");
    let sample = (tile.x + tile.width * 3 / 4, tile.y + tile.height * 3 / 4);
    assert!(
        window_bounds(source.handle()).is_some_and(|bounds| same_rect(bounds, desktop)),
        "the fullscreen fixture collapsed before FrigoTab opened"
    );
    assert!(application.send_marked_test_key(VK_LEFT_ALT, false));
    assert!(application.send_marked_test_key(VK_TAB, false));
    assert!(application.wait_visible(TIMEOUT));
    assert!(application.send_marked_test_key(VK_TAB, true));
    assert!(application.send_marked_test_key(VK_LEFT_ALT, true));
    assert!(wait_until(TIMEOUT, || foreground_window() == application.owner()));
    assert!(
        wait_until(TIMEOUT, || {
            window_bounds(source.handle()).is_some_and(|bounds| {
                bounds.right - bounds.left == 1 && bounds.bottom - bounds.top == 1
            })
        }),
        "the real fullscreen fixture did not discard its presentation surface"
    );

    let deadline = Instant::now() + TIMEOUT;
    let mut observed = Vec::new();
    while Instant::now() < deadline {
        unsafe {
            let _ = DwmFlush();
        }
        if let Some(color) = screen_pixel(sample.0, sample.1) {
            observed.push(color);
            if color_distance(color, GREEN) <= 55 {
                break;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        observed
            .last()
            .is_some_and(|color| color_distance(*color, GREEN) <= 55),
        "the fullscreen source lost its preview after its live surface collapsed: {observed:#x?}"
    );

    let digit = 0x31 + source_index as u16;
    assert!(application.send_marked_test_key(digit, false));
    assert!(application.wait_hidden(TIMEOUT));
    assert!(application.send_marked_test_key(digit, true));
    assert!(wait_until(TIMEOUT, || foreground_window() == source.handle()));
    assert!(
        wait_until(TIMEOUT, || {
            window_bounds(source.handle()).is_some_and(|bounds| same_rect(bounds, desktop))
        }),
        "numbered selection did not restore the real fullscreen source"
    );
}

fn same_rect(
    left: windows_sys::Win32::Foundation::RECT,
    right: windows_sys::Win32::Foundation::RECT,
) -> bool {
    left.left == right.left
        && left.top == right.top
        && left.right == right.right
        && left.bottom == right.bottom
}
