#![cfg(windows)]

use std::time::Duration;

use frigotab::key_handling::KeyHandling;
use frigotab::keyboard_input::{KeyTransition, KeyboardInput, SwitcherKey};
use frigotab::screen_point::ScreenPoint;
use frigotab::switcher_state::SwitcherState;
use frigotab_acceptance::{
    LiveSession, LiveTile, foreground_window, is_layered, is_window_visible, serial_guard,
    visible_owned_layered_windows, wait_until, window_bounds, window_owner,
};
use windows_sys::Win32::Foundation::{HWND, RECT};

const TIMEOUT: Duration = Duration::from_secs(5);

fn open_session(session: &mut LiveSession) {
    let result = session.open();
    assert_eq!(KeyHandling::Consume, result);
    assert!(
        session.wait_visible(TIMEOUT),
        "the real session did not become visible"
    );
    assert!(
        !session.tiles().is_empty(),
        "the real session has no preview HWNDs"
    );
    assert_eq!(SwitcherState::Visible, session.switcher_state());
    assert_eq!(session.candidate_count(), session.tiles().len());
    assert_eq!(Some(0), session.selected_index());
}

fn key(
    session: &mut LiveSession,
    key: SwitcherKey,
    transition: KeyTransition,
    alt: bool,
    shift: bool,
    injected: bool,
) -> KeyHandling {
    session.key(KeyboardInput::new(key, transition, alt, shift, injected))
}

fn center(tile: LiveTile) -> ScreenPoint {
    ScreenPoint::new(
        tile.bounds.left + (tile.bounds.right - tile.bounds.left) / 2,
        tile.bounds.top + (tile.bounds.bottom - tile.bounds.top) / 2,
    )
}

fn fixture_tiles(session: &LiveSession) -> Vec<LiveTile> {
    let fixtures = session.fixture_handles();
    session
        .tiles()
        .into_iter()
        .filter(|tile| fixtures.contains(&tile.source))
        .collect()
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

fn wait_foreground(expected: HWND) {
    assert!(
        wait_until(TIMEOUT, || foreground_window() == expected),
        "the selected real target did not become foreground"
    );
}

fn assert_hidden(session: &mut LiveSession) {
    assert!(
        wait_until(TIMEOUT, || {
            session.switcher_state() == SwitcherState::Idle && !is_window_visible(session.owner())
        }),
        "the real session did not close"
    );
    assert!(
        visible_owned_layered_windows(session.owner()).is_empty(),
        "a preview HWND remained visible after closing the session"
    );
    assert_eq!(SwitcherState::Idle, session.switcher_state());
    assert_eq!(0, session.candidate_count());
    assert_eq!(None, session.selected_index());
}

#[test]
fn alt_tab_opens_the_real_switcher_and_selects_a_real_preview() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();

    open_session(&mut session);

    assert!(is_window_visible(session.owner()));
    assert!(session.selected_tile().is_some());
    assert_eq!(1, session.selected_count());
    assert!(fixture_tiles(&session).len() >= 3);

    let tiles = session.tiles();
    let overlays = visible_owned_layered_windows(session.owner());
    assert_eq!(tiles.len(), overlays.len());
    for tile in tiles {
        assert!(is_window_visible(tile.popup));
        assert_eq!(session.owner(), window_owner(tile.popup));
        assert!(is_layered(tile.popup));
    }
}

#[test]
fn holding_alt_cycles_forward_and_wraps_through_real_previews() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);

    let sources: Vec<HWND> = session
        .tiles()
        .into_iter()
        .map(|tile| tile.source)
        .collect();
    assert!(sources.len() >= 3);
    assert_eq!(Some(sources[0]), session.selected_source());

    for expected in sources.iter().skip(1) {
        let index = sources
            .iter()
            .position(|source| source == expected)
            .expect("the selected source disappeared from the real tile list");
        assert_eq!(
            KeyHandling::Consume,
            key(
                &mut session,
                SwitcherKey::Tab,
                KeyTransition::Down,
                true,
                false,
                false
            )
        );
        assert_eq!(Some(*expected), session.selected_source());
        assert_eq!(Some(index), session.selected_index());
        assert!(session.is_visible());
    }
    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Tab,
            KeyTransition::Down,
            true,
            false,
            false
        )
    );
    assert_eq!(Some(sources[0]), session.selected_source());
    assert_eq!(Some(0), session.selected_index());
}

#[test]
fn shift_alt_tab_cycles_backward_and_wraps_through_real_previews() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);

    let sources: Vec<HWND> = session
        .tiles()
        .into_iter()
        .map(|tile| tile.source)
        .collect();
    assert!(sources.len() >= 3);
    let last = sources.len() - 1;

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Tab,
            KeyTransition::Down,
            true,
            true,
            false
        )
    );
    assert_eq!(Some(sources[last]), session.selected_source());
    assert_eq!(Some(last), session.selected_index());
    for (index, expected) in sources[..last].iter().enumerate().rev() {
        assert_eq!(
            KeyHandling::Consume,
            key(
                &mut session,
                SwitcherKey::Tab,
                KeyTransition::Down,
                true,
                true,
                false
            )
        );
        assert_eq!(Some(*expected), session.selected_source());
        assert_eq!(Some(index), session.selected_index());
    }
}

#[test]
fn releasing_alt_leaves_the_real_switcher_open_for_deliberate_selection() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);

    let initial = session.selected_source();
    let (target_index, target) = session
        .tiles()
        .into_iter()
        .enumerate()
        .find(|(_, tile)| Some(tile.source) != initial)
        .expect("at least two real previews are required");
    session.mouse_move(center(target));
    assert_eq!(Some(target.source), session.selected_source());
    assert_eq!(Some(target_index), session.selected_index());

    assert_eq!(
        KeyHandling::PassThrough,
        key(
            &mut session,
            SwitcherKey::Alt,
            KeyTransition::Up,
            false,
            false,
            false
        )
    );
    assert!(session.is_visible());
    assert!(is_window_visible(session.owner()));
    assert_eq!(Some(target.source), session.selected_source());
    assert_eq!(Some(target_index), session.selected_index());

    session.mouse_click(center(target));
    assert_hidden(&mut session);
    wait_foreground(target.source);
}

#[test]
fn escape_cancels_the_real_session() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Escape,
            KeyTransition::Down,
            false,
            false,
            false
        )
    );
    assert_hidden(&mut session);
}

#[test]
fn alt_f4_cancels_the_real_session() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::F4,
            KeyTransition::Down,
            true,
            false,
            false
        )
    );
    assert_hidden(&mut session);
}

#[test]
fn number_one_activates_the_first_real_preview() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);
    let expected = session
        .selected_source()
        .expect("the first real preview was not selected");
    assert_eq!(Some(0), session.selected_index());

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::D1,
            KeyTransition::Down,
            false,
            false,
            false
        )
    );
    assert_hidden(&mut session);
    wait_foreground(expected);
}

#[test]
fn number_pad_two_activates_the_second_real_preview() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
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
            false
        )
    );
    let expected = session
        .selected_source()
        .expect("Tab did not select a second preview");
    assert_eq!(Some(sources[1]), Some(expected));
    assert_eq!(Some(1), session.selected_index());
    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Tab,
            KeyTransition::Down,
            true,
            true,
            false
        )
    );
    assert_eq!(Some(sources[0]), session.selected_source());
    assert_eq!(Some(0), session.selected_index());

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::NumPad2,
            KeyTransition::Down,
            false,
            false,
            false,
        )
    );
    assert_hidden(&mut session);
    wait_foreground(expected);
}

#[test]
fn pointer_hover_and_click_select_and_activate_the_exact_real_preview() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);
    let initial = session.selected_source();
    let target = fixture_tiles(&session)
        .into_iter()
        .find(|tile| Some(tile.source) != initial)
        .expect("a second real fixture preview is required");

    session.mouse_move(center(target));
    assert_eq!(Some(target.source), session.selected_source());
    assert_eq!(
        Some(
            session
                .tiles()
                .iter()
                .position(|tile| tile.source == target.source)
                .expect("the hovered fixture tile disappeared")
        ),
        session.selected_index()
    );
    session.mouse_click(center(target));

    assert_hidden(&mut session);
    wait_foreground(target.source);
}

#[test]
fn moving_outside_the_tiles_clears_selection_and_alt_tab_restores_it() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);
    let tiles = session.tiles();
    let first = session.selected_source();
    let outside = point_outside_tiles(&tiles, window_bounds(session.owner()).unwrap());

    session.mouse_move(outside);
    assert_eq!(None, session.selected_source());
    assert_eq!(None, session.selected_index());
    assert_eq!(0, session.selected_count());
    assert!(session.is_visible());
    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Tab,
            KeyTransition::Down,
            true,
            false,
            false
        )
    );
    assert_eq!(first, session.selected_source());
    assert_eq!(Some(0), session.selected_index());
    assert_eq!(1, session.selected_count());
}

#[test]
fn a_closed_target_leaves_the_real_session_available_for_cancellation() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);
    let target = fixture_tiles(&session)
        .into_iter()
        .next()
        .expect("a real fixture preview is required");

    session.mouse_move(center(target));
    assert_eq!(Some(target.source), session.selected_source());
    assert!(session.selected_index().is_some());
    assert!(session.close_fixture(target.source));
    assert_eq!(
        KeyHandling::PassThrough,
        key(
            &mut session,
            SwitcherKey::Alt,
            KeyTransition::Up,
            false,
            false,
            false
        )
    );
    assert!(session.is_visible());
    assert!(is_window_visible(session.owner()));
    assert!(session.selected_index().is_some());

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Escape,
            KeyTransition::Down,
            false,
            false,
            false
        )
    );
    assert_hidden(&mut session);
}

#[test]
fn closing_and_reopening_recreates_real_native_previews() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);
    let count = session.tiles().len();
    assert!(count > 0);

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Escape,
            KeyTransition::Down,
            false,
            false,
            false
        )
    );
    assert_hidden(&mut session);
    assert!(session.tiles().is_empty());

    open_session(&mut session);
    assert_eq!(count, session.tiles().len());
    assert!(session.selected_tile().is_some());
    assert_eq!(Some(0), session.selected_index());
}

#[test]
fn injected_alt_tab_passes_through_without_opening_real_windows() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();

    assert_eq!(
        KeyHandling::PassThrough,
        key(
            &mut session,
            SwitcherKey::Tab,
            KeyTransition::Down,
            true,
            false,
            true
        )
    );
    assert!(!session.is_visible());
    assert!(!is_window_visible(session.owner()));
    assert!(session.tiles().is_empty());
    assert_eq!(SwitcherState::Idle, session.switcher_state());
    assert_eq!(0, session.candidate_count());
    assert_eq!(None, session.selected_index());
}

#[test]
fn consumed_key_ups_remain_balanced_after_the_real_session_closes() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();
    open_session(&mut session);
    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Escape,
            KeyTransition::Down,
            false,
            false,
            false
        )
    );
    assert_hidden(&mut session);

    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Tab,
            KeyTransition::Up,
            false,
            false,
            false
        )
    );
    assert_eq!(
        KeyHandling::Consume,
        key(
            &mut session,
            SwitcherKey::Escape,
            KeyTransition::Up,
            false,
            false,
            false
        )
    );
    assert_eq!(
        KeyHandling::PassThrough,
        key(
            &mut session,
            SwitcherKey::Tab,
            KeyTransition::Up,
            false,
            false,
            false
        )
    );
    assert!(!session.is_visible());
}

#[test]
fn repeated_open_and_close_leaves_no_preview_forms_behind() {
    let _serial = serial_guard();
    let mut session = LiveSession::new();

    for _ in 0..5 {
        open_session(&mut session);
        assert!(!session.tiles().is_empty());
        assert_eq!(
            KeyHandling::Consume,
            key(
                &mut session,
                SwitcherKey::Escape,
                KeyTransition::Down,
                false,
                false,
                false
            )
        );
        assert_hidden(&mut session);
        assert!(session.tiles().is_empty());
    }
}
