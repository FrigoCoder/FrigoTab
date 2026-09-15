#![cfg(windows)]

//! Real-process acceptance coverage for the first composed DWM preview frame.
//! A preview popup must not become observable as a black placeholder while
//! DWM is still preparing the registered source thumbnail.

use std::{
    thread,
    time::{Duration, Instant},
};

use frigotab::layout::Layout;
use frigotab::window_finder::WindowFinder;
use frigotab::window_handle::WindowHandle;
use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, GREEN, MAGENTA, RED, RunningFrigoTab, color_distance,
    screen_pixel, serial_guard, set_per_monitor_dpi_awareness,
};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::WindowsAndMessaging::{WS_EX_APPWINDOW, WS_POPUP};

const TIMEOUT: Duration = Duration::from_secs(3);

#[test]
fn a_real_thumbnail_is_ready_on_its_first_composed_owner_frame() {
    let _guard = serial_guard();
    set_per_monitor_dpi_awareness();
    let fixtures = create_fixtures();
    let application = RunningFrigoTab::start().expect("FrigoTab.exe should start");
    let finder = WindowFinder::new();
    let layout = Layout::new(&finder.windows);
    let green_bounds = layout
        .bounds
        .get(&WindowHandle::new(fixtures[0].handle()))
        .copied()
        .expect("the green fixture should have production layout bounds");
    let sample = (
        green_bounds.x + green_bounds.width * 3 / 4,
        green_bounds.y + green_bounds.height * 3 / 4,
    );

    let started = Instant::now();
    assert!(application.open(), "the real session should accept open");

    let deadline = started + TIMEOUT;
    let mut history = Vec::new();
    while Instant::now() < deadline {
        if application.is_visible() {
            // Synchronize with the compositor rather than merely observing the
            // HWND style. This samples each actually publishable owner frame.
            unsafe {
                let _ = DwmFlush();
            }
            if let Some(color) = screen_pixel(sample.0, sample.1) {
                history.push((started.elapsed(), color));
                if near(GREEN, color, 55) {
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(1));
    }

    let reveal = history
        .last()
        .map(|(elapsed, _)| *elapsed)
        .expect("the visible owner published no preview frames");
    assert!(
        history
            .last()
            .is_some_and(|(_, color)| near(GREEN, *color, 55)),
        "the green real DWM thumbnail did not become visible: {history:?}"
    );
    eprintln!(
        "green thumbnail reveal: {reveal:?}; frames: {:?}",
        history
            .iter()
            .map(|(elapsed, color)| format!("{elapsed:?}={color:#08x}"))
            .collect::<Vec<_>>()
    );

    let first_color = history[0].1;
    assert!(
        near(GREEN, first_color, 55),
        "the first composed owner frame contained a placeholder instead of the real thumbnail: {history:?}"
    );
}

fn create_fixtures() -> Vec<FixtureWindow> {
    let options = |left: i32| FixtureOptions {
        bounds: RECT {
            left,
            top: 100,
            right: left + 640,
            bottom: 580,
        },
        style: WS_POPUP,
        ex_style: WS_EX_APPWINDOW,
        activate: true,
        ..FixtureOptions::default()
    };
    vec![
        FixtureWindow::show_with_options("FrigoTab reveal green", GREEN, options(80)),
        FixtureWindow::show_with_options("FrigoTab reveal red", RED, options(760)),
        FixtureWindow::show_with_options("FrigoTab reveal magenta", MAGENTA, options(1440)),
    ]
}

fn near(expected: u32, actual: u32, tolerance: u32) -> bool {
    actual != u32::MAX && color_distance(expected, actual) <= tolerance
}
