#![cfg(windows)]

//! Black-box acceptance coverage for source windows disappearing independently
//! of FrigoTab. The shipped executable must refresh its visible preview graph
//! even when no close button initiated the destruction.

use std::thread;
use std::time::Duration;

use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, GREEN, MAGENTA, RunningFrigoTab, ScreenCapture,
    capture_screen_image, color_distance, foreground_window, serial_guard, wait_until,
    window_bounds,
};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindow, WS_EX_APPWINDOW, WS_POPUP};

const TIMEOUT: Duration = Duration::from_secs(5);
const VK_LEFT_ALT_KEY: u16 = 0xa4;
const VK_TAB_KEY: u16 = 0x09;
const VK_ESCAPE_KEY: u16 = 0x1b;

#[test]
fn an_externally_closed_application_is_removed_from_the_visible_session() {
    let _serial = serial_guard();
    let mut fixtures = real_fixtures();
    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab did not start in marked-input acceptance mode");

    open_session(&application);
    let target_popup = wait_for_preview(&application, GREEN);
    let _survivor = wait_for_preview(&application, MAGENTA);
    let initial_overlay_count = application.visible_owned_layered_windows().len();

    fixtures[1].hide();
    thread::sleep(Duration::from_millis(350));
    assert!(
        is_valid_window(target_popup),
        "temporarily hiding a source destroyed its live preview"
    );
    assert_eq!(
        initial_overlay_count,
        application.visible_owned_layered_windows().len(),
        "temporarily hiding a source rebuilt the preview graph"
    );
    assert!(application.is_visible());
    fixtures[1].show_again();
    assert!(
        wait_until(TIMEOUT, || {
            wait_for_preview_optional(&application, GREEN).is_some()
        }),
        "the existing preview did not recover after its source was shown again"
    );

    fixtures[1].close();

    let refreshed = wait_until(TIMEOUT, || {
        !is_valid_window(target_popup)
            && wait_for_preview_optional(&application, GREEN).is_none()
            && application.visible_owned_layered_windows().len() < initial_overlay_count
    });
    assert!(
        refreshed,
        "an externally destroyed source left its stale preview in the live session: \
         target_valid={}, green_visible={}, overlays={}/{}, session_visible={}, foreground={:?}",
        is_valid_window(target_popup),
        wait_for_preview_optional(&application, GREEN).is_some(),
        application.visible_owned_layered_windows().len(),
        initial_overlay_count,
        application.is_visible(),
        foreground_window(),
    );
    assert!(
        application.is_visible(),
        "refreshing an externally closed source ended the sticky session"
    );
    assert_eq!(application.owner(), foreground_window());
    assert!(
        wait_for_preview_optional(&application, MAGENTA).is_some(),
        "refreshing the preview graph removed a surviving application"
    );

    close_session(&application);
}

fn real_fixtures() -> Vec<FixtureWindow> {
    [MAGENTA, GREEN]
        .into_iter()
        .enumerate()
        .map(|(index, color)| {
            FixtureWindow::show_with_options(
                &format!("FrigoTab external lifecycle {}", index + 1),
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

fn wait_for_preview(application: &RunningFrigoTab, color: u32) -> HWND {
    let mut preview = None;
    assert!(
        wait_until(TIMEOUT, || {
            preview = wait_for_preview_optional(application, color);
            preview.is_some()
        }),
        "the real preview for color {color:#08x} did not become visible"
    );
    preview.expect("preview became unavailable after the wait")
}

fn wait_for_preview_optional(application: &RunningFrigoTab, color: u32) -> Option<HWND> {
    unsafe {
        let _ = DwmFlush();
    }
    application
        .visible_owned_layered_windows()
        .into_iter()
        .filter_map(|popup| {
            let bounds = window_bounds(popup)?;
            let image = capture_screen_image(bounds)?;
            Some((preview_color_samples(&image, bounds, color), popup))
        })
        .max_by_key(|(samples, _)| *samples)
        .filter(|(samples, _)| *samples >= 100)
        .map(|(_, popup)| popup)
}

fn preview_color_samples(image: &ScreenCapture, bounds: RECT, color: u32) -> usize {
    let selected = selected_overlay_color(color);
    (bounds.top..bounds.bottom)
        .step_by(4)
        .flat_map(|y| (bounds.left..bounds.right).step_by(4).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            image.pixel(x, y).is_some_and(|pixel| {
                color_distance(pixel, color) <= 55 || color_distance(pixel, selected) <= 55
            })
        })
        .count()
}

fn selected_overlay_color(color: u32) -> u32 {
    let red = (color & 0xff) / 2;
    let green = ((color >> 8) & 0xff) / 2;
    let blue = (((color >> 16) & 0xff) + 255) / 2;
    red | (green << 8) | (blue << 16)
}

fn is_valid_window(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 }
}
