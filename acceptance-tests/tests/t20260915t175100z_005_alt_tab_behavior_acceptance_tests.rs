#![cfg(windows)]

use std::time::Duration;

use frigotab::key_handling::KeyHandling;
use frigotab::keyboard_input::{KeyTransition, KeyboardInput, SwitcherKey};
use frigotab::screen_point::ScreenPoint;
use frigotab::switcher_application::AltTabBehavior;
use frigotab_acceptance::{
    LiveSession, LiveTile, foreground_window, serial_guard, wait_until, window_bounds,
};
use windows_sys::Win32::Foundation::{HWND, RECT};

const TIMEOUT: Duration = Duration::from_secs(5);

fn open_session(session: &mut LiveSession) {
    assert_eq!(
        KeyHandling::Consume,
        session.open(),
        "Alt+Tab did not open the real switcher"
    );
    assert!(
        session.wait_visible(TIMEOUT),
        "the real session did not become visible"
    );
    assert!(
        !session.tiles().is_empty(),
        "the real session has no previews"
    );
}

fn key(
    session: &mut LiveSession,
    key: SwitcherKey,
    transition: KeyTransition,
    alt: bool,
    shift: bool,
) -> KeyHandling {
    session.key(KeyboardInput::new(key, transition, alt, shift, false))
}

fn wait_foreground(expected: HWND) {
    assert!(
        wait_until(TIMEOUT, || foreground_window() == expected),
        "the selected real target did not become foreground"
    );
}

fn point_outside_tiles(tiles: &[LiveTile], bounds: RECT) -> ScreenPoint {
    for y in (bounds.top..bounds.bottom).step_by(8) {
        for x in (bounds.left..bounds.right).step_by(8) {
            if tiles.iter().all(|tile| {
                x < tile.bounds.left
                    || x >= tile.bounds.right
                    || y < tile.bounds.top
                    || y >= tile.bounds.bottom
            }) {
                return ScreenPoint::new(x, y);
            }
        }
    }
    panic!("no screen point outside the real preview HWNDs was available");
}

#[test]
fn default_sticky_behavior_keeps_the_real_session_open_after_alt_release() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();

    assert_eq!(AltTabBehavior::Sticky, session.alt_tab_behavior());
    open_session(&mut session);
    let selected = session.selected_source();

    assert_eq!(
        KeyHandling::PassThrough,
        key(
            &mut session,
            SwitcherKey::Alt,
            KeyTransition::Up,
            false,
            false,
        )
    );
    assert!(session.is_visible());
    assert_eq!(selected, session.selected_source());

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Escape,
            KeyTransition::Down,
            false,
            false,
        )
    );
}

#[test]
fn tap_behavior_activates_the_initial_real_preview_on_alt_release() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    session.set_alt_tab_behavior(AltTabBehavior::Tap);
    open_session(&mut session);
    let expected = session
        .selected_source()
        .expect("the initial real preview was not selected");

    assert_eq!(
        KeyHandling::PassThrough,
        key(
            &mut session,
            SwitcherKey::Alt,
            KeyTransition::Up,
            false,
            false,
        )
    );
    assert!(session.wait_hidden(TIMEOUT));
    wait_foreground(expected);
}

#[test]
fn tap_behavior_activates_the_forward_selection_on_alt_release() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    session.set_alt_tab_behavior(AltTabBehavior::Tap);
    open_session(&mut session);
    let sources: Vec<HWND> = session
        .tiles()
        .into_iter()
        .map(|tile| tile.source)
        .collect();
    assert!(sources.len() >= 2);

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Tab,
            KeyTransition::Down,
            true,
            false,
        )
    );
    assert_eq!(Some(sources[1]), session.selected_source());
    assert_eq!(
        KeyHandling::PassThrough,
        key(
            &mut session,
            SwitcherKey::Alt,
            KeyTransition::Up,
            false,
            false,
        )
    );
    assert!(session.wait_hidden(TIMEOUT));
    wait_foreground(sources[1]);
}

#[test]
fn tap_behavior_activates_the_reverse_selection_on_alt_release() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    session.set_alt_tab_behavior(AltTabBehavior::Tap);
    open_session(&mut session);
    let sources: Vec<HWND> = session
        .tiles()
        .into_iter()
        .map(|tile| tile.source)
        .collect();
    assert!(sources.len() >= 2);
    let expected = *sources.last().expect("the real preview list was empty");

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Tab,
            KeyTransition::Down,
            true,
            true,
        )
    );
    assert_eq!(Some(expected), session.selected_source());
    assert_eq!(
        KeyHandling::PassThrough,
        key(
            &mut session,
            SwitcherKey::Alt,
            KeyTransition::Up,
            false,
            false,
        )
    );
    assert!(session.wait_hidden(TIMEOUT));
    wait_foreground(expected);
}

#[test]
fn tap_release_without_a_selection_keeps_the_real_session_available() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    session.set_alt_tab_behavior(AltTabBehavior::Tap);
    open_session(&mut session);
    let tiles = session.tiles();
    let outside = point_outside_tiles(
        &tiles,
        window_bounds(session.owner()).expect("the real session owner has no bounds"),
    );
    session.mouse_move(outside);
    assert_eq!(None, session.selected_source());

    assert_eq!(
        KeyHandling::PassThrough,
        key(
            &mut session,
            SwitcherKey::Alt,
            KeyTransition::Up,
            false,
            false,
        )
    );
    assert!(session.is_visible());
    assert_eq!(None, session.selected_source());

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Escape,
            KeyTransition::Down,
            false,
            false,
        )
    );
}
