#![cfg(windows)]

//! Real-process coverage for fullscreen windows whose live surfaces turn black
//! after FrigoTab has cached them. An ordinary window remains a live DWM source.

use std::thread;
use std::time::{Duration, Instant};

use frigotab::geometry::{Layout, virtual_screen_bounds};
use frigotab::window::{WindowFinder, WindowHandle};
use frigotab_acceptance::{
    BLUE, FixtureOptions, FixtureWindow, GREEN, MAGENTA, ORANGE, RunningFrigoTab,
    capture_screen_image, color_distance, foreground_window, screen_pixel, serial_guard,
    use_per_monitor_physical_coordinates, visible_owned_layered_windows, wait_until, window_bounds,
};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::WindowsAndMessaging::{WS_EX_APPWINDOW, WS_EX_TOPMOST, WS_POPUP};

const VK_LEFT_ALT: u16 = 0xa4;
const VK_TAB: u16 = 0x09;
const KEY_SEND_LIMIT: Duration = Duration::from_millis(75);
const UI_READY_LIMIT: Duration = Duration::from_secs(3);
const UI_PERFORMANCE_LIMIT: Duration = Duration::from_millis(750);
// DWM/color-management conversion can move solid fixture colors noticeably
// when a cached frame crosses monitors with different output profiles.
const COLOR_TOLERANCE: u32 = 110;

#[test]
fn fullscreen_thumbnails_remain_identifiable_after_three_foreground_handoffs() {
    let _serial = serial_guard();
    assert!(use_per_monitor_physical_coordinates());
    let desktop = virtual_screen_bounds();
    assert!(desktop.right > desktop.left && desktop.bottom > desktop.top);

    // Keep an ordinary, live DWM source available underneath the fullscreen
    // handoffs. The final fullscreen fixture remains foreground when the
    // switcher is opened.
    let ordinary = FixtureWindow::show_with_options(
        "FrigoTab ordinary live preview",
        GREEN,
        FixtureOptions {
            bounds: RECT {
                left: desktop.left + 100,
                top: desktop.top + 100,
                right: desktop.left + 740,
                bottom: desktop.top + 580,
            },
            style: WS_POPUP,
            ex_style: WS_EX_APPWINDOW,
            activate: true,
            ..FixtureOptions::default()
        },
    );
    assert!(WindowHandle::new(ordinary.handle()).set_foreground());
    assert!(wait_until(Duration::from_secs(2), || {
        foreground_window() == ordinary.handle()
    }));

    let application = RunningFrigoTab::start_accepting_marked_test_input()
        .expect("FrigoTab.exe should start in marked-input acceptance mode");

    // FrigoTab is resident before these sources appear. Each fullscreen HWND
    // is foreground long enough for the application-side preview cache to
    // observe it, then its real client surface turns black before handoff.
    let fullscreen = [
        ("FrigoTab fullscreen preview red", MAGENTA),
        ("FrigoTab fullscreen preview green", GREEN),
        ("FrigoTab fullscreen preview blue", BLUE),
    ]
    .into_iter()
    .map(|(title, color)| {
        let fixture = FixtureWindow::show_with_options(
            title,
            color,
            FixtureOptions {
                bounds: desktop,
                style: WS_POPUP,
                ex_style: WS_EX_APPWINDOW | WS_EX_TOPMOST,
                activate: true,
                ..FixtureOptions::default()
            },
        );
        assert!(WindowHandle::new(fixture.handle()).set_foreground());
        assert!(wait_until(Duration::from_secs(2), || {
            foreground_window() == fixture.handle()
        }));
        assert!(
            wait_until(Duration::from_secs(2), || {
                screen_pixel(
                    desktop.left + (desktop.right - desktop.left) / 2,
                    desktop.top + (desktop.bottom - desktop.top) / 2,
                )
                .is_some_and(|pixel| color_distance(pixel, color) <= COLOR_TOLERANCE)
            }),
            "foreground fullscreen fixture did not paint its expected color"
        );
        // Leave room for two low-rate observations without exposing any
        // implementation hook or synchronizing on private cache state.
        thread::sleep(Duration::from_millis(1_100));
        fixture.set_color(0);
        unsafe {
            let _ = DwmFlush();
        }
        fixture
    })
    .collect::<Vec<_>>();

    for fixture in &fullscreen {
        assert!(window_bounds(fixture.handle()).is_some_and(|bounds| same_rect(bounds, desktop)));
    }
    assert!(window_bounds(ordinary.handle()).is_some_and(|bounds| {
        bounds.right - bounds.left >= 600 && bounds.bottom - bounds.top >= 400
    }));

    let finder = WindowFinder::new();
    let handles: Vec<_> = fullscreen
        .iter()
        .map(|fixture| WindowHandle::new(fixture.handle()))
        .chain(std::iter::once(WindowHandle::new(ordinary.handle())))
        .collect();
    let indices: Vec<_> = handles
        .iter()
        .map(|handle| {
            finder
                .windows
                .iter()
                .position(|candidate| candidate == handle)
                .expect("every fixture must be a real Alt-Tab candidate")
        })
        .collect();
    assert!(
        indices.iter().all(|index| *index < 9),
        "the real fixtures must remain addressable by D1..D9: {indices:?}"
    );
    let layout = Layout::new(&finder.windows);
    let tile_bounds: Vec<_> = handles
        .iter()
        .map(|handle| {
            let tile = layout
                .bounds
                .get(handle)
                .expect("each real fixture should have preview bounds");
            assert!(tile.width >= 100 && tile.height >= 100);
            RECT {
                left: tile.x,
                top: tile.y,
                right: tile.x + tile.width,
                bottom: tile.y + tile.height,
            }
        })
        .collect();
    let tile_samples: Vec<_> = tile_bounds
        .iter()
        .map(|tile| {
            (
                tile.left + (tile.right - tile.left) * 3 / 4,
                tile.top + (tile.bottom - tile.top) / 2,
            )
        })
        .collect();

    assert_eq!(foreground_window(), fullscreen[2].handle());
    let started = Instant::now();
    let alt_start = Instant::now();
    assert!(application.send_marked_test_key(VK_LEFT_ALT, false));
    let alt_send = alt_start.elapsed();
    let tab_start = Instant::now();
    assert!(application.send_marked_test_key(VK_TAB, false));
    let tab_send = tab_start.elapsed();
    assert!(alt_send < KEY_SEND_LIMIT, "Alt hook send took {alt_send:?}");
    assert!(tab_send < KEY_SEND_LIMIT, "Tab hook send took {tab_send:?}");

    assert!(
        application.wait_visible(UI_READY_LIMIT),
        "the switcher owner did not become visible within {UI_READY_LIMIT:?}"
    );
    assert!(application.send_marked_test_key(VK_TAB, true));
    assert!(application.send_marked_test_key(VK_LEFT_ALT, true));
    let previews_ready = || {
        foreground_window() == application.owner()
            && application.visible_owned_layered_windows().len() >= handles.len()
            && tile_bounds.iter().all(|expected| {
                visible_owned_layered_windows(application.owner())
                    .into_iter()
                    .filter_map(window_bounds)
                    .any(|actual| same_rect(actual, *expected))
            })
    };
    assert!(
        wait_until(UI_READY_LIMIT, previews_ready),
        "the real preview windows did not become ready in time"
    );
    let ui_ready = started.elapsed();
    assert!(
        ui_ready < UI_PERFORMANCE_LIMIT,
        "session and preview construction took {ui_ready:?}, over {UI_PERFORMANCE_LIMIT:?}"
    );

    let fullscreen_colors = [MAGENTA, GREEN, BLUE];
    let visuals_ready = wait_until(UI_READY_LIMIT, || {
        unsafe {
            let _ = DwmFlush();
        }
        fullscreen_colors
            .iter()
            .enumerate()
            .all(|(index, color)| tile_is_cached_or_labeled_black(tile_bounds[index], *color))
            && fullscreen_colors
                .iter()
                .enumerate()
                .any(|(index, color)| tile_has_cached_color(tile_bounds[index], *color))
            && screen_pixel(tile_samples[3].0, tile_samples[3].1).is_some_and(|pixel| {
                color_distance(pixel, GREEN) <= COLOR_TOLERANCE
                    || color_distance(pixel, selected_overlay_color(GREEN)) <= COLOR_TOLERANCE
            })
    });
    if !visuals_ready {
        let samples: Vec<_> = tile_samples
            .iter()
            .enumerate()
            .map(|(index, point)| {
                (
                    index,
                    screen_pixel(point.0, point.1),
                    fallback_label_has_visible_text(tile_bounds[index]),
                )
            })
            .collect();
        panic!(
            "each fullscreen tile must show its cached color or a labeled black fallback; the ordinary tile must remain live green; samples={samples:?}"
        );
    }

    // Activate the oldest cached source. This is the one most likely to have
    // been lost by a cache that only remembers the immediately previous app.
    let oldest_index = WindowFinder::new()
        .windows
        .iter()
        .position(|candidate| *candidate == WindowHandle::new(fullscreen[0].handle()))
        .expect("the oldest fullscreen source must remain a numbered candidate");
    assert!(oldest_index < 9);
    assert!(application.send_marked_test_key(0x31 + oldest_index as u16, false));
    assert!(application.send_marked_test_key(0x31 + oldest_index as u16, true));
    assert!(
        application.wait_hidden(UI_READY_LIMIT)
            && wait_until(UI_READY_LIMIT, || {
                foreground_window() == fullscreen[0].handle()
                    && window_bounds(fullscreen[0].handle())
                        .is_some_and(|bounds| same_rect(bounds, desktop))
            }),
        "numbered activation did not restore the oldest fullscreen candidate"
    );

    // Once the same real HWND becomes windowed, the resident observer must
    // discard its frozen fullscreen frame and let DWM provide a live preview.
    let windowed_bounds = RECT {
        left: desktop.left + 160,
        top: desktop.top + 120,
        right: desktop.left + 800,
        bottom: desktop.top + 600,
    };
    assert!(fullscreen[0].set_bounds(windowed_bounds));
    fullscreen[0].set_color(ORANGE);
    assert!(WindowHandle::new(fullscreen[0].handle()).set_foreground());
    assert!(wait_until(Duration::from_secs(2), || {
        foreground_window() == fullscreen[0].handle()
            && window_bounds(fullscreen[0].handle())
                .is_some_and(|bounds| same_rect(bounds, windowed_bounds))
    }));
    thread::sleep(Duration::from_millis(1_100));

    let windowed_finder = WindowFinder::new();
    let windowed_layout = Layout::new(&windowed_finder.windows);
    let source = WindowHandle::new(fullscreen[0].handle());
    let tile = windowed_layout
        .bounds
        .get(&source)
        .expect("windowed source should retain a real preview tile");
    let sample = (tile.x + tile.width * 3 / 4, tile.y + tile.height / 2);
    assert!(application.send_marked_test_key(VK_LEFT_ALT, false));
    assert!(application.send_marked_test_key(VK_TAB, false));
    assert!(application.wait_visible(UI_READY_LIMIT));
    assert!(application.send_marked_test_key(VK_TAB, true));
    assert!(application.send_marked_test_key(VK_LEFT_ALT, true));
    assert!(
        wait_until(UI_READY_LIMIT, || {
            unsafe {
                let _ = DwmFlush();
            }
            screen_pixel(sample.0, sample.1).is_some_and(|pixel| {
                color_distance(pixel, ORANGE) <= COLOR_TOLERANCE
                    || color_distance(pixel, selected_overlay_color(ORANGE)) <= COLOR_TOLERANCE
            })
        }),
        "a source which returned to windowed mode kept its frozen fullscreen preview"
    );
}

fn tile_is_cached_or_labeled_black(tile: RECT, cached_color: u32) -> bool {
    tile_has_cached_color(tile, cached_color) || tile_is_labeled_black(tile)
}

fn tile_has_cached_color(tile: RECT, cached_color: u32) -> bool {
    let width = tile.right - tile.left;
    let height = tile.bottom - tile.top;
    let body_points = [
        (tile.left + width / 4, tile.top + height * 2 / 5),
        (tile.left + width * 3 / 4, tile.top + height * 2 / 5),
        (tile.left + width / 4, tile.top + height * 3 / 5),
        (tile.left + width * 3 / 4, tile.top + height * 3 / 5),
    ];
    let Some(pixels) = body_points
        .into_iter()
        .map(|(x, y)| screen_pixel(x, y))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    pixels.iter().all(|pixel| {
        color_distance(*pixel, cached_color) <= COLOR_TOLERANCE
            || color_distance(*pixel, selected_overlay_color(cached_color)) <= COLOR_TOLERANCE
    })
}

fn tile_is_labeled_black(tile: RECT) -> bool {
    let width = tile.right - tile.left;
    let height = tile.bottom - tile.top;
    let body_points = [
        (tile.left + width / 4, tile.top + height * 2 / 5),
        (tile.left + width * 3 / 4, tile.top + height * 2 / 5),
        (tile.left + width / 4, tile.top + height * 3 / 5),
        (tile.left + width * 3 / 4, tile.top + height * 3 / 5),
    ];
    let Some(pixels) = body_points
        .into_iter()
        .map(|(x, y)| screen_pixel(x, y))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    pixels.iter().all(|pixel| {
        color_distance(*pixel, 0) <= 35 || color_distance(*pixel, selected_overlay_color(0)) <= 35
    }) && fallback_label_has_visible_text(tile)
}

fn fallback_label_has_visible_text(tile: RECT) -> bool {
    const LABEL_TOP: f32 = 0.63;
    const LABEL_BOTTOM: f32 = 0.84;
    let bounds = RECT {
        left: tile.left + (tile.right - tile.left) / 4,
        top: tile.top + ((tile.bottom - tile.top) as f32 * LABEL_TOP) as i32,
        right: tile.left + (tile.right - tile.left) * 3 / 4,
        bottom: tile.top + ((tile.bottom - tile.top) as f32 * LABEL_BOTTOM) as i32,
    };
    let Some(image) = capture_screen_image(bounds) else {
        return false;
    };
    let mut bright_pixels = 0;
    for y in bounds.top..bounds.bottom {
        for x in bounds.left..bounds.right {
            if image
                .pixel(x, y)
                .is_some_and(|pixel| color_distance(pixel, 0x00ff_ffff) <= COLOR_TOLERANCE)
            {
                bright_pixels += 1;
            }
        }
    }
    bright_pixels >= 20
}

fn selected_overlay_color(color: u32) -> u32 {
    let red = (color & 0xff) / 2;
    let green = ((color >> 8) & 0xff) / 2;
    let blue = (((color >> 16) & 0xff) + 255) / 2;
    red | (green << 8) | (blue << 16)
}

fn same_rect(left: RECT, right: RECT) -> bool {
    left.left == right.left
        && left.top == right.top
        && left.right == right.right
        && left.bottom == right.bottom
}
