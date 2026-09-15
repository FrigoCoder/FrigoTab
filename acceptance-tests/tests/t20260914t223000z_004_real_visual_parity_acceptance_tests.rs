#![cfg(windows)]

//! Real visual acceptance checks for the Rust executable.
//!
//! These tests deliberately start the built executable and inspect its real
//! owned HWNDs and composed screen pixels.  They do not construct a substitute
//! switcher graph or use test-only production hooks.

use std::time::Duration;

use frigotab_acceptance::{
    FixtureOptions, FixtureWindow, GREEN, MAGENTA, RED, RunningFrigoTab, ScreenCapture,
    capture_screen_image, color_distance, is_window_visible, pump_messages, screen_pixel,
    serial_guard, set_per_monitor_dpi_awareness, wait_until, window_bounds, window_ex_style,
    window_owner,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PostMessageW, WM_MOUSEMOVE, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

const TILE_EXTENDED_STYLES: u32 =
    WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_NOACTIVATE;

#[test]
fn every_real_preview_uses_the_original_owned_layered_window_topology() {
    let _guard = serial_guard();
    set_per_monitor_dpi_awareness();
    let _fixtures = create_visual_fixtures();
    let session = RunningFrigoTab::start().expect("FrigoTab.exe should start");

    assert!(
        session.open(),
        "the real session owner should accept WM_BEGIN_SESSION"
    );
    assert!(
        session.wait_visible(Duration::from_secs(5)),
        "the real owner should become visible"
    );
    assert!(
        wait_until(Duration::from_secs(5), || session
            .visible_owned_layered_windows()
            .len()
            >= 3),
        "the real session should create one layered tile for each fixture"
    );

    post_owner_mouse_point(session.owner(), 1, 1);
    let mut fixture_tiles = vec![
        wait_for_tile_color(&session, RED).0,
        wait_for_tile_color(&session, GREEN).0,
        wait_for_tile_color(&session, MAGENTA).0,
    ];
    fixture_tiles.sort_by_key(|tile| *tile as usize);
    fixture_tiles.dedup();
    assert_eq!(
        3,
        fixture_tiles.len(),
        "the real session did not expose exactly the three fixture previews"
    );
    for tile in fixture_tiles {
        assert!(is_window_visible(tile), "every real tile should be visible");
        assert_eq!(
            window_owner(tile),
            session.owner(),
            "every tile should be owned by the session owner"
        );
        assert_eq!(
            window_ex_style(tile) & TILE_EXTENDED_STYLES,
            TILE_EXTENDED_STYLES,
            "tile HWND did not preserve the original tool/topmost/transparent/layered/no-activate styles"
        );
    }
}

#[test]
fn dwm_preview_fills_the_real_tile_and_selected_tile_keeps_the_blue_alpha_overlay() {
    let _guard = serial_guard();
    set_per_monitor_dpi_awareness();
    let _fixtures = create_visual_fixtures();
    let session = RunningFrigoTab::start().expect("FrigoTab.exe should start");

    assert!(
        session.open(),
        "the real session owner should accept WM_BEGIN_SESSION"
    );
    assert!(session.wait_visible(Duration::from_secs(5)));
    post_owner_mouse_point(session.owner(), 1, 1);
    let (red_tile, red_bounds) = wait_for_tile_color(&session, RED);
    let (_green_tile, green_bounds) = wait_for_tile_color(&session, GREEN);

    let red_sample = tile_sample(red_bounds);
    let green_sample = tile_sample(green_bounds);
    assert!(
        near(RED, red_sample, 55),
        "red DWM source did not fill its tile: {red_sample:#08x}"
    );
    assert!(
        near(GREEN, green_sample, 55),
        "green DWM source did not fill its tile: {green_sample:#08x}"
    );
    let edge_y = green_bounds.top + (green_bounds.bottom - green_bounds.top) * 3 / 4;
    let left_edge = screen_pixel(green_bounds.left + 1, edge_y).unwrap_or(u32::MAX);
    let right_edge = screen_pixel(green_bounds.right - 2, edge_y).unwrap_or(u32::MAX);
    assert!(
        near(GREEN, left_edge, 55),
        "the DWM image did not reach the tile's left edge: {left_edge:#08x}"
    );
    assert!(
        near(GREEN, right_edge, 55),
        "the DWM image did not reach the tile's right edge: {right_edge:#08x}"
    );

    post_mouse_move(session.owner(), red_bounds);
    let expected_blue_blend = frigotab_acceptance::rgb(127, 0, 128);
    assert!(
        wait_until(Duration::from_secs(5), || {
            near(tile_sample(red_bounds), expected_blue_blend, 75)
        }),
        "the selected real tile lost the original half-transparent blue overlay"
    );
    assert!(
        near(tile_sample(green_bounds), GREEN, 55),
        "an unselected real preview was unexpectedly tinted"
    );

    // Keep the handle live through the assertion so the test documents that
    // the selected tile was found as a real owned window, not just a pixel.
    assert!(!red_tile.is_null());
}

#[test]
fn real_overlay_keeps_the_measured_title_and_large_centered_number() {
    let _guard = serial_guard();
    set_per_monitor_dpi_awareness();
    let _fixtures = create_visual_fixtures();
    let session = RunningFrigoTab::start().expect("FrigoTab.exe should start");

    assert!(
        session.open(),
        "the real session owner should accept WM_BEGIN_SESSION"
    );
    assert!(session.wait_visible(Duration::from_secs(5)));
    post_owner_mouse_point(session.owner(), 1, 1);
    let (_red_tile, red_bounds) = wait_for_tile_color(&session, RED);
    post_mouse_move(session.owner(), red_bounds);
    assert!(wait_until(Duration::from_secs(5), || {
        near(
            tile_sample(red_bounds),
            frigotab_acceptance::rgb(127, 0, 128),
            75,
        )
    }));

    let image = capture_screen_image(red_bounds).expect("the tile should be capturable");
    assert!(image.width() > 0 && image.height() > 0);
    assert!(
        near(
            frigotab_acceptance::rgb(0, 0, 0),
            image
                .pixel(red_bounds.left + 2, red_bounds.top + 2)
                .unwrap_or(u32::MAX),
            35
        ),
        "the measured title backing no longer starts at the tile origin"
    );
    assert!(
        !near(
            frigotab_acceptance::rgb(0, 0, 0),
            image
                .pixel(red_bounds.right - 5, red_bounds.top + 5)
                .unwrap_or(u32::MAX),
            45
        ),
        "the title backing was changed into a full-width header"
    );

    let title_right = red_bounds.left + 4 + (220).min(image.width() - 8);
    let title_bottom = red_bounds.top + 4 + (60).min(image.height() - 8);
    let title_white = count_pixels(
        &image,
        red_bounds,
        red_bounds.left + 4,
        red_bounds.top + 4,
        title_right,
        title_bottom,
        is_white,
    );
    assert!(
        title_white > 8,
        "the real icon/title overlay did not render in the upper-left corner"
    );

    let number_left = red_bounds.left + image.width() / 4;
    let number_top = red_bounds.top + image.height() / 4;
    let number_right = red_bounds.left + image.width() * 3 / 4;
    let number_bottom = red_bounds.top + image.height() * 3 / 4;
    let white_number = find_white_pixels(
        &image,
        red_bounds,
        number_left,
        number_top,
        number_right,
        number_bottom,
    );
    assert!(
        white_number.len() > 20,
        "the centered number has no visible white glyph"
    );
    let min_y = white_number.iter().map(|(_, y)| *y).min().unwrap();
    let max_y = white_number.iter().map(|(_, y)| *y).max().unwrap();
    assert!(
        max_y - min_y >= 35,
        "the centered number is no longer rendered with the original large font"
    );
    let number_black = count_pixels(
        &image,
        red_bounds,
        number_left,
        number_top,
        number_right,
        number_bottom,
        is_black,
    );
    assert!(
        number_black > 100,
        "the centered number lost its tight black backing rectangle"
    );
}

fn create_visual_fixtures() -> Vec<FixtureWindow> {
    let options = |left: i32, top: i32| FixtureOptions {
        bounds: RECT {
            left,
            top,
            right: left + 640,
            bottom: top + 480,
        },
        style: WS_POPUP,
        ex_style: WS_EX_APPWINDOW,
        activate: true,
        ..Default::default()
    };
    let fixtures = vec![
        FixtureWindow::show_with_options("R", RED, options(80, 80)),
        FixtureWindow::show_with_options("G", GREEN, options(760, 120)),
        FixtureWindow::show_with_options("M", frigotab_acceptance::MAGENTA, options(1440, 160)),
    ];
    pump_messages();
    fixtures
}

fn wait_for_tile_color(session: &RunningFrigoTab, expected: u32) -> (HWND, RECT) {
    let mut found = None;
    assert!(
        wait_until(Duration::from_secs(5), || {
            found = session
                .visible_owned_layered_windows()
                .into_iter()
                .filter_map(|tile| window_bounds(tile).map(|bounds| (tile, bounds)))
                .find(|(_, bounds)| near(tile_sample(*bounds), expected, 55));
            found.is_some()
        }),
        "the real DWM preview did not expose the expected fixture color"
    );
    found.expect("wait_until reported a missing tile")
}

fn tile_sample(bounds: RECT) -> u32 {
    flush_composition();
    let x = bounds.left + (bounds.right - bounds.left) * 3 / 4;
    let y = bounds.top + (bounds.bottom - bounds.top) * 3 / 4;
    screen_pixel(x, y).unwrap_or(u32::MAX)
}

fn post_mouse_move(owner: HWND, tile: RECT) {
    let owner_bounds = window_bounds(owner).expect("session owner should have screen bounds");
    let x = tile.left + (tile.right - tile.left) / 2 - owner_bounds.left;
    let y = tile.top + (tile.bottom - tile.top) / 2 - owner_bounds.top;
    let packed = ((y as i16 as u16 as u32) << 16) | (x as i16 as u16 as u32);
    assert!(unsafe { PostMessageW(owner, WM_MOUSEMOVE, 0, packed as LPARAM) } != 0);
    pump_messages();
}

fn post_owner_mouse_point(owner: HWND, x: i32, y: i32) {
    let packed = ((y as i16 as u16 as u32) << 16) | (x as i16 as u16 as u32);
    assert!(unsafe { PostMessageW(owner, WM_MOUSEMOVE, 0, packed as LPARAM) } != 0);
    pump_messages();
}

fn capture_pixel_count<F>(
    image: &ScreenCapture,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    predicate: F,
) -> usize
where
    F: Fn(u32) -> bool,
{
    let mut count = 0;
    for y in top..bottom {
        for x in left..right {
            if image.pixel(x, y).is_some_and(&predicate) {
                count += 1;
            }
        }
    }
    count
}

fn count_pixels<F>(
    image: &ScreenCapture,
    _bounds: RECT,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    predicate: F,
) -> usize
where
    F: Fn(u32) -> bool,
{
    capture_pixel_count(image, left, top, right, bottom, predicate)
}

fn find_white_pixels(
    image: &ScreenCapture,
    _bounds: RECT,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> Vec<(i32, i32)> {
    let mut result = Vec::new();
    for y in top..bottom {
        for x in left..right {
            if image.pixel(x, y).is_some_and(is_white) {
                result.push((x, y));
            }
        }
    }
    result
}

fn flush_composition() {
    unsafe {
        let _ = DwmFlush();
    }
}

fn near(expected: u32, actual: u32, tolerance: u32) -> bool {
    actual != u32::MAX && color_distance(expected, actual) <= tolerance
}

fn is_white(color: u32) -> bool {
    color & 0xff >= 210 && (color >> 8) & 0xff >= 210 && (color >> 16) & 0xff >= 210
}

fn is_black(color: u32) -> bool {
    color & 0xff <= 35 && (color >> 8) & 0xff <= 35 && (color >> 16) & 0xff <= 35
}
