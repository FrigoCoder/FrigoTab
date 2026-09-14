use frigo_tab_core::{
    AltTabRecoveryPlan, DeferredKeyboardDispatcher, DispatchTicket, GridLayout, KeyHandling,
    KeyTransition, KeyboardInput, KeyboardModifierKey, KeyboardModifierState,
    KeyboardSuppressionState, LayoutMonitor, LayoutWindow, PortError, ScreenPoint, ScreenRectangle,
    SwitcherApplication, SwitcherKey, SwitcherSessionPort, SwitcherState,
};

#[derive(Default)]
struct FakePort {
    candidate_count: usize,
    open_result: bool,
    activation_result: bool,
    fail_open: bool,
    fail_select: bool,
    fail_hit_test: bool,
    fail_clear: bool,
    fail_activate: bool,
    fail_relayout: bool,
    fail_close: bool,
    hit_test_result: Option<usize>,
    open_calls: usize,
    close_calls: usize,
    activate_calls: usize,
    relayout_calls: usize,
    selections: Vec<usize>,
    activations: Vec<Option<usize>>,
    current_selection: Option<usize>,
}

impl FakePort {
    fn with_candidates(candidate_count: usize) -> Self {
        Self {
            candidate_count,
            open_result: true,
            activation_result: true,
            ..Self::default()
        }
    }
}

impl SwitcherSessionPort for FakePort {
    fn try_open(&mut self) -> Result<usize, PortError> {
        self.open_calls += 1;
        if self.fail_open {
            return Err(PortError::new("open failed"));
        }
        Ok(if self.open_result {
            self.candidate_count
        } else {
            0
        })
    }

    fn select(&mut self, index: usize) -> Result<(), PortError> {
        if self.fail_select {
            return Err(PortError::new("select failed"));
        }
        self.current_selection = Some(index);
        self.selections.push(index);
        Ok(())
    }

    fn clear_selection(&mut self) -> Result<(), PortError> {
        if self.fail_clear {
            return Err(PortError::new("clear failed"));
        }
        self.current_selection = None;
        Ok(())
    }

    fn hit_test(&mut self, _point: ScreenPoint) -> Result<Option<usize>, PortError> {
        if self.fail_hit_test {
            return Err(PortError::new("hit test failed"));
        }
        Ok(self.hit_test_result)
    }

    fn try_activate_selected(&mut self) -> Result<bool, PortError> {
        self.activate_calls += 1;
        self.activations.push(self.current_selection);
        if self.fail_activate {
            return Err(PortError::new("activation failed"));
        }
        Ok(self.activation_result)
    }

    fn close(&mut self) -> Result<(), PortError> {
        self.close_calls += 1;
        self.current_selection = None;
        if self.fail_close {
            return Err(PortError::new("close failed"));
        }
        Ok(())
    }

    fn relayout(&mut self) -> Result<(), PortError> {
        self.relayout_calls += 1;
        if self.fail_relayout {
            return Err(PortError::new("relayout failed"));
        }
        Ok(())
    }
}

fn input(key: SwitcherKey, transition: KeyTransition, alt: bool, shift: bool) -> KeyboardInput {
    KeyboardInput::new(key, transition, alt, shift, false)
}

fn down(key: SwitcherKey, alt: bool, shift: bool) -> KeyboardInput {
    input(key, KeyTransition::Down, alt, shift)
}

fn up(key: SwitcherKey, alt: bool) -> KeyboardInput {
    input(key, KeyTransition::Up, alt, false)
}

#[test]
fn first_alt_tab_opens_and_selects_first_candidate() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));

    assert_eq!(
        application.handle_keyboard(down(SwitcherKey::Tab, true, false)),
        KeyHandling::Consume
    );
    assert_eq!(application.state(), SwitcherState::Visible);
    assert_eq!(application.selected_index(), Some(0));
    assert_eq!(application.port().open_calls, 1);
    assert_eq!(application.port().selections, vec![0]);
}

#[test]
fn repeated_alt_tab_moves_forward_and_wraps() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    for _ in 0..4 {
        application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    }

    assert_eq!(application.selected_index(), Some(0));
    assert_eq!(application.port().selections, vec![0, 1, 2, 0]);
}

#[test]
fn shift_alt_tab_moves_backwards_and_wraps() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_keyboard(down(SwitcherKey::Tab, true, true));
    application.handle_keyboard(down(SwitcherKey::Tab, true, true));

    assert_eq!(application.selected_index(), Some(1));
}

#[test]
fn releasing_alt_keeps_session_visible_until_number_selection() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::Alt, false)),
        KeyHandling::PassThrough
    );
    assert_eq!(application.state(), SwitcherState::Visible);
    assert_eq!(
        application.handle_keyboard(down(SwitcherKey::D2, false, false)),
        KeyHandling::Consume
    );
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().activations, vec![Some(1)]);
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::D2, false)),
        KeyHandling::Consume
    );
}

#[test]
fn empty_candidate_list_fails_open_and_cleans_up() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(0));

    assert_eq!(
        application.handle_keyboard(down(SwitcherKey::Tab, true, false)),
        KeyHandling::PassThrough
    );
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().close_calls, 1);
}

#[test]
fn close_actions_consume_matching_key_up_after_session_ends() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));

    assert_eq!(
        application.handle_keyboard(down(SwitcherKey::Escape, false, false)),
        KeyHandling::Consume
    );
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::Escape, false)),
        KeyHandling::Consume
    );
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::Tab, true)),
        KeyHandling::Consume
    );
}

#[test]
fn failed_activation_leaves_session_available_for_retry() {
    let mut port = FakePort::with_candidates(3);
    port.activation_result = false;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_keyboard(down(SwitcherKey::D1, false, false));

    assert_eq!(application.state(), SwitcherState::Visible);
    assert_eq!(application.port().activate_calls, 1);
    assert_eq!(application.port().close_calls, 0);
}

#[test]
fn pointer_selection_and_clear_follow_the_same_session_policy() {
    let mut port = FakePort::with_candidates(3);
    port.hit_test_result = None;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_mouse_move(ScreenPoint::new(100, 100));
    assert_eq!(application.selected_index(), None);

    application.port_mut().hit_test_result = Some(2);
    application.handle_mouse_click(ScreenPoint::new(100, 100));
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().activations, vec![Some(2)]);
}

#[test]
fn port_failure_interrupts_and_clears_state() {
    let mut port = FakePort::with_candidates(3);
    port.fail_relayout = true;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.relayout();

    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().close_calls, 1);
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::Tab, true)),
        KeyHandling::PassThrough
    );
}

#[test]
fn modifier_state_uses_event_transitions_and_keeps_the_other_shift_key() {
    let mut state = KeyboardModifierState::new();
    let alt_down = state.create_input(SwitcherKey::Alt, KeyTransition::Down, false, false);
    let tab_down = state.create_input(SwitcherKey::Tab, KeyTransition::Down, false, false);
    let alt_up = state.create_input(SwitcherKey::Alt, KeyTransition::Up, false, false);
    let later_tab = state.create_input(SwitcherKey::Tab, KeyTransition::Down, false, false);
    assert!(alt_down.alt);
    assert!(tab_down.alt);
    assert!(alt_up.alt);
    assert!(!later_tab.alt);

    state.create_input_with_modifier(
        SwitcherKey::Shift,
        KeyTransition::Down,
        false,
        false,
        KeyboardModifierKey::LeftShift,
    );
    state.create_input_with_modifier(
        SwitcherKey::Shift,
        KeyTransition::Down,
        false,
        false,
        KeyboardModifierKey::RightShift,
    );
    state.create_input_with_modifier(
        SwitcherKey::Shift,
        KeyTransition::Up,
        false,
        false,
        KeyboardModifierKey::LeftShift,
    );
    assert!(state.shift_down());
}

#[test]
fn suppression_balances_consumed_keys_and_pending_admission() {
    let mut state = KeyboardSuppressionState::new();
    let tab_down = down(SwitcherKey::Tab, true, false);
    let tab_up = up(SwitcherKey::Tab, true);
    let (consumed, token) = state.should_consume_with_admission(tab_down);
    assert!(consumed);
    assert_ne!(token, 0);
    assert!(state.admission_pending());
    state.set_session_visible(false);
    assert!(state.should_consume(tab_up));
    assert!(!state.should_consume(tab_up));

    let (consumed, second_token) = state.should_consume_with_admission(tab_down);
    assert!(consumed);
    assert_ne!(second_token, 0);
    assert!(state.abort_pending_admission(second_token));
    assert!(!state.session_active_or_pending());
}

#[test]
fn recovery_plan_replays_a_complete_forward_and_reverse_gesture() {
    let forward = AltTabRecoveryPlan::create(false, false, false);
    assert_eq!(forward.len(), 4);
    assert_eq!(
        forward[0],
        KeyboardInput::new(SwitcherKey::Alt, KeyTransition::Down, true, false, true)
    );
    assert_eq!(
        forward[1],
        KeyboardInput::new(SwitcherKey::Tab, KeyTransition::Down, true, false, true)
    );
    assert_eq!(
        forward[2],
        KeyboardInput::new(SwitcherKey::Tab, KeyTransition::Up, true, false, true)
    );
    assert_eq!(
        forward[3],
        KeyboardInput::new(SwitcherKey::Alt, KeyTransition::Up, true, false, true)
    );

    let reverse = AltTabRecoveryPlan::create(false, true, false);
    assert_eq!(reverse.len(), 6);
    assert_eq!(reverse[1].key, SwitcherKey::Shift);
    assert_eq!(reverse[1].transition, KeyTransition::Down);
}

#[test]
fn dispatcher_defers_delivery_bounds_queue_and_invalidates_stale_events() {
    let mut dispatcher = DeferredKeyboardDispatcher::new(1).unwrap();
    let event = down(SwitcherKey::Tab, true, false);
    let ticket = dispatcher
        .try_dispatch(event)
        .expect("ordinary slot is available");
    assert!(dispatcher.try_dispatch(event).is_none());
    dispatcher.invalidate_pending();
    assert!(!ticket.is_current(dispatcher.generation()));
    assert_eq!(ticket.input(), event);
    assert!(dispatcher.complete(ticket));
    assert_eq!(dispatcher.pending_count(), 0);
}

#[test]
fn grid_layout_keeps_aspect_ratio_margin_and_monitor_origins() {
    let monitors = vec![
        LayoutMonitor::new(
            "left",
            ScreenRectangle::new(-1920, 0, 1920, 1080),
            ScreenRectangle::new(-1920, 0, 1920, 1040),
        )
        .unwrap(),
        LayoutMonitor::new(
            "primary",
            ScreenRectangle::new(0, 0, 1920, 1080),
            ScreenRectangle::new(0, 0, 1920, 1040),
        )
        .unwrap(),
    ];
    let windows = vec![
        LayoutWindow::new(
            "leftApp",
            "left",
            ScreenRectangle::new(-1800, 100, 800, 600),
        )
        .unwrap(),
        LayoutWindow::new(
            "rightApp",
            "primary",
            ScreenRectangle::new(100, 100, 1600, 900),
        )
        .unwrap(),
    ];
    let tiles = GridLayout::new().arrange(&windows, &monitors);
    let left = tiles["leftApp"];
    let right = tiles["rightApp"];
    assert!(left.x < 0 || left.y < 0);
    assert!(left.x >= monitors[0].working_area.x);
    assert!(left.right() <= monitors[0].working_area.right());
    assert!(right.x > monitors[1].working_area.x);
    assert!(right.right() < monitors[1].working_area.right());
    let source_ratio = 1600.0 / 900.0;
    let tile_ratio = right.width as f64 / right.height as f64;
    assert!((source_ratio - tile_ratio).abs() <= 0.02);
}

#[test]
fn exception_while_opening_fails_open_and_cleans_up() {
    let mut port = FakePort::with_candidates(3);
    port.fail_open = true;
    let mut application = SwitcherApplication::new(port);

    assert_eq!(
        application.handle_keyboard(down(SwitcherKey::Tab, true, false)),
        KeyHandling::PassThrough
    );
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().close_calls, 1);
}

#[test]
fn exception_while_selecting_initial_candidate_fails_open_and_cleans_up() {
    let mut port = FakePort::with_candidates(3);
    port.fail_select = true;
    let mut application = SwitcherApplication::new(port);

    assert_eq!(
        application.handle_keyboard(down(SwitcherKey::Tab, true, false)),
        KeyHandling::PassThrough
    );
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().close_calls, 1);
}

#[test]
fn unrelated_key_is_passed_through_while_idle() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    assert_eq!(
        application.handle_keyboard(down(SwitcherKey::Unknown, false, false)),
        KeyHandling::PassThrough
    );
    assert_eq!(application.port().open_calls, 0);
}

#[test]
fn injected_alt_tab_is_passed_through() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    let injected = KeyboardInput::new(SwitcherKey::Tab, KeyTransition::Down, true, false, true);
    assert_eq!(
        application.handle_keyboard(injected),
        KeyHandling::PassThrough
    );
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().open_calls, 0);
}

#[test]
fn key_up_event_is_passed_through_while_idle() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::Tab, true)),
        KeyHandling::PassThrough
    );
    assert_eq!(application.port().open_calls, 0);
}

#[test]
fn escape_cancels_without_activation() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    assert_eq!(
        application.handle_keyboard(down(SwitcherKey::Escape, false, false)),
        KeyHandling::Consume
    );
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().activate_calls, 0);
    assert_eq!(application.port().close_calls, 1);
}

#[test]
fn alt_f4_cancels_without_activation() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    assert_eq!(
        application.handle_keyboard(down(SwitcherKey::F4, true, false)),
        KeyHandling::Consume
    );
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().activate_calls, 0);
    assert_eq!(application.port().close_calls, 1);
}

#[test]
fn d1_selects_and_activates_the_first_candidate() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_keyboard(down(SwitcherKey::D1, false, false));
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().activations, vec![Some(0)]);
}

#[test]
fn numpad_two_selects_and_activates_the_second_candidate() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_keyboard(down(SwitcherKey::NumPad2, false, false));
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().activations, vec![Some(1)]);
}

#[test]
fn invalid_digit_preserves_the_current_selection() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    assert_eq!(
        application.handle_keyboard(down(SwitcherKey::D9, false, false)),
        KeyHandling::Consume
    );
    assert_eq!(application.state(), SwitcherState::Visible);
    assert_eq!(application.selected_index(), Some(0));
    assert_eq!(application.port().activate_calls, 0);
}

#[test]
fn pointer_hover_selects_the_candidate_under_the_pointer() {
    let mut port = FakePort::with_candidates(3);
    port.hit_test_result = Some(2);
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_mouse_move(ScreenPoint::new(100, 100));
    assert_eq!(application.selected_index(), Some(2));
}

#[test]
fn pointer_click_activates_the_candidate_under_the_pointer() {
    let mut port = FakePort::with_candidates(3);
    port.hit_test_result = Some(1);
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_mouse_click(ScreenPoint::new(100, 100));
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().activations, vec![Some(1)]);
}

#[test]
fn pointer_outside_every_tile_clears_selection_without_activation() {
    let mut port = FakePort::with_candidates(3);
    port.hit_test_result = None;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_mouse_move(ScreenPoint::new(100, 100));
    assert_eq!(application.selected_index(), None);
    assert_eq!(application.port().activate_calls, 0);
}

#[test]
fn alt_tab_restores_keyboard_selection_after_pointer_clears_it() {
    let mut port = FakePort::with_candidates(3);
    port.hit_test_result = None;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_mouse_move(ScreenPoint::new(100, 100));
    assert_eq!(application.selected_index(), None);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    assert_eq!(application.selected_index(), Some(0));
}

#[test]
fn shift_alt_tab_restores_the_last_selection_after_pointer_clears_it() {
    let mut port = FakePort::with_candidates(3);
    port.hit_test_result = None;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_mouse_move(ScreenPoint::new(100, 100));
    application.handle_keyboard(down(SwitcherKey::Tab, true, true));
    assert_eq!(application.selected_index(), Some(2));
}

#[test]
fn invalid_pointer_candidate_closes_the_inconsistent_session_safely() {
    let mut port = FakePort::with_candidates(3);
    port.hit_test_result = Some(99);
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_mouse_click(ScreenPoint::new(100, 100));
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().close_calls, 1);
    assert_eq!(application.port().activate_calls, 0);
}

#[test]
fn hit_test_failure_closes_the_session_safely() {
    let mut port = FakePort::with_candidates(3);
    port.fail_hit_test = true;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_mouse_move(ScreenPoint::new(100, 100));
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().close_calls, 1);
}

#[test]
fn selection_clear_failure_closes_the_session_safely() {
    let mut port = FakePort::with_candidates(3);
    port.hit_test_result = None;
    port.fail_clear = true;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_mouse_move(ScreenPoint::new(100, 100));
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().close_calls, 1);
}

#[test]
fn activation_exception_leaves_the_session_visible_for_retry_or_cancel() {
    let mut port = FakePort::with_candidates(3);
    port.fail_activate = true;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_keyboard(down(SwitcherKey::D1, false, false));
    assert_eq!(application.state(), SwitcherState::Visible);
    assert_eq!(application.port().activate_calls, 1);
    assert_eq!(application.port().close_calls, 0);
}

#[test]
fn interruption_returns_the_application_to_idle() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.interrupt();
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().close_calls, 1);
}

#[test]
fn cleanup_exception_cannot_leave_the_application_active() {
    let mut port = FakePort::with_candidates(3);
    port.fail_close = true;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_keyboard(down(SwitcherKey::Escape, false, false));
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(application.port().close_calls, 1);
}

#[test]
fn display_change_relayouts_a_visible_session() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.relayout();
    assert_eq!(application.state(), SwitcherState::Visible);
    assert_eq!(application.port().relayout_calls, 1);
}

#[test]
fn relayout_failure_clears_consumed_keys_from_the_interrupted_session() {
    let mut port = FakePort::with_candidates(3);
    port.fail_relayout = true;
    let mut application = SwitcherApplication::new(port);
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.relayout();
    assert_eq!(application.state(), SwitcherState::Idle);
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::Tab, true)),
        KeyHandling::PassThrough
    );
}

#[test]
fn closed_session_can_be_opened_again() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_keyboard(down(SwitcherKey::Escape, false, false));
    application.handle_keyboard(up(SwitcherKey::Escape, false));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    assert_eq!(application.state(), SwitcherState::Visible);
    assert_eq!(application.port().open_calls, 2);
}

#[test]
fn consumed_alt_tab_key_down_also_consumes_its_tab_key_up() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::Tab, true)),
        KeyHandling::Consume
    );
}

#[test]
fn consumed_digit_key_down_also_consumes_its_key_up() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_keyboard(down(SwitcherKey::D1, false, false));
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::D1, false)),
        KeyHandling::Consume
    );
}

#[test]
fn consumed_escape_key_down_also_consumes_its_key_up() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_keyboard(down(SwitcherKey::Escape, false, false));
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::Escape, false)),
        KeyHandling::Consume
    );
}

#[test]
fn consumed_alt_f4_key_down_also_consumes_its_f4_key_up() {
    let mut application = SwitcherApplication::new(FakePort::with_candidates(3));
    application.handle_keyboard(down(SwitcherKey::Tab, true, false));
    application.handle_keyboard(down(SwitcherKey::F4, true, false));
    assert_eq!(
        application.handle_keyboard(up(SwitcherKey::F4, true)),
        KeyHandling::Consume
    );
}

#[test]
fn four_windows_use_a_near_square_grid() {
    let monitor = LayoutMonitor::new(
        "primary",
        ScreenRectangle::new(0, 0, 1920, 1080),
        ScreenRectangle::new(0, 0, 1920, 1040),
    )
    .unwrap();
    let windows = vec![
        LayoutWindow::new("one", "primary", ScreenRectangle::new(0, 0, 800, 600)).unwrap(),
        LayoutWindow::new("two", "primary", ScreenRectangle::new(0, 0, 800, 600)).unwrap(),
        LayoutWindow::new("three", "primary", ScreenRectangle::new(0, 0, 800, 600)).unwrap(),
        LayoutWindow::new("four", "primary", ScreenRectangle::new(0, 0, 800, 600)).unwrap(),
    ];
    let tiles = GridLayout::new().arrange(&windows, &[monitor]);
    let x = windows
        .iter()
        .map(|window| tiles[&window.id].x)
        .collect::<std::collections::HashSet<_>>();
    let y = windows
        .iter()
        .map(|window| tiles[&window.id].y)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(x.len(), 2);
    assert_eq!(y.len(), 2);
    assert_eq!(tiles.len(), 4);
}

#[test]
fn wide_source_window_keeps_its_aspect_ratio() {
    let monitor = LayoutMonitor::new(
        "primary",
        ScreenRectangle::new(0, 0, 1000, 800),
        ScreenRectangle::new(0, 0, 1000, 800),
    )
    .unwrap();
    let windows =
        vec![LayoutWindow::new("wide", "primary", ScreenRectangle::new(0, 0, 1600, 900)).unwrap()];
    let tile = GridLayout::new().arrange(&windows, &[monitor])["wide"];
    let source_ratio = 1600.0 / 900.0;
    let tile_ratio = tile.width as f64 / tile.height as f64;
    assert!((source_ratio - tile_ratio).abs() <= 0.02);
}

#[test]
fn tiles_retain_a_margin_from_monitor_edges() {
    let monitor = LayoutMonitor::new(
        "primary",
        ScreenRectangle::new(0, 0, 1000, 1000),
        ScreenRectangle::new(0, 0, 1000, 1000),
    )
    .unwrap();
    let windows =
        vec![
            LayoutWindow::new("square", "primary", ScreenRectangle::new(0, 0, 1000, 1000)).unwrap(),
        ];
    let tile = GridLayout::new().arrange(&windows, &[monitor])["square"];
    assert!(tile.x > 0);
    assert!(tile.y > 0);
    assert!(tile.right() < 1000);
    assert!(tile.bottom() < 1000);
}

#[test]
fn each_monitor_receives_its_own_independent_grid() {
    let monitors = vec![
        LayoutMonitor::new(
            "left",
            ScreenRectangle::new(-1920, 0, 1920, 1080),
            ScreenRectangle::new(-1920, 0, 1920, 1040),
        )
        .unwrap(),
        LayoutMonitor::new(
            "primary",
            ScreenRectangle::new(0, 0, 1920, 1080),
            ScreenRectangle::new(0, 0, 1920, 1040),
        )
        .unwrap(),
    ];
    let windows = vec![
        LayoutWindow::new(
            "leftApp",
            "left",
            ScreenRectangle::new(-1800, 100, 800, 600),
        )
        .unwrap(),
        LayoutWindow::new(
            "rightApp",
            "primary",
            ScreenRectangle::new(100, 100, 800, 600),
        )
        .unwrap(),
    ];
    let tiles = GridLayout::new().arrange(&windows, &monitors);
    let left = tiles["leftApp"];
    let right = tiles["rightApp"];
    assert!(left.x < 0 || left.y < 0);
    assert!(left.x >= monitors[0].working_area.x);
    assert!(left.right() <= monitors[0].working_area.right());
    assert!(right.x >= monitors[1].working_area.x);
    assert!(right.right() <= monitors[1].working_area.right());
}

#[test]
fn invalid_source_dimensions_are_ignored() {
    let monitor = LayoutMonitor::new(
        "primary",
        ScreenRectangle::new(0, 0, 1920, 1080),
        ScreenRectangle::new(0, 0, 1920, 1040),
    )
    .unwrap();
    let windows = vec![
        LayoutWindow::new("zeroWidth", "primary", ScreenRectangle::new(0, 0, 0, 600)).unwrap(),
        LayoutWindow::new(
            "negativeHeight",
            "primary",
            ScreenRectangle::new(0, 0, 800, -1),
        )
        .unwrap(),
    ];
    assert!(GridLayout::new().arrange(&windows, &[monitor]).is_empty());
}

#[test]
fn shift_state_comes_from_event_transitions() {
    let mut state = KeyboardModifierState::new();
    state.create_input(SwitcherKey::Shift, KeyTransition::Down, false, false);
    let tab_down = state.create_input(SwitcherKey::Tab, KeyTransition::Down, true, false);
    let shift_up = state.create_input(SwitcherKey::Shift, KeyTransition::Up, true, false);
    let later_tab = state.create_input(SwitcherKey::Tab, KeyTransition::Down, true, false);
    assert!(tab_down.shift);
    assert!(shift_up.shift);
    assert!(!later_tab.shift);
}

#[test]
fn injected_modifiers_do_not_change_physical_state() {
    let mut state = KeyboardModifierState::new();
    state.create_input(SwitcherKey::Alt, KeyTransition::Down, false, true);
    let physical_tab = state.create_input(SwitcherKey::Tab, KeyTransition::Down, false, false);
    assert!(!physical_tab.alt);
    assert!(!state.alt_down());
}

#[test]
fn suppression_balances_a_key_after_session_close() {
    let mut state = KeyboardSuppressionState::new();
    let tab_down = down(SwitcherKey::Tab, true, false);
    let tab_up = up(SwitcherKey::Tab, true);
    assert!(state.should_consume(tab_down));
    state.set_session_visible(false);
    assert!(state.should_consume(tab_up));
    assert!(!state.should_consume(tab_up));
}

#[test]
fn auto_repeat_needs_only_one_matching_release() {
    let mut state = KeyboardSuppressionState::new();
    state.set_session_visible(true);
    let down_digit = down(SwitcherKey::D1, false, false);
    let up_digit = up(SwitcherKey::D1, false);
    assert!(state.should_consume(down_digit));
    assert!(state.should_consume(down_digit));
    assert!(state.should_consume(up_digit));
    assert!(!state.should_consume(up_digit));
}

#[test]
fn failed_open_replays_a_complete_gesture_after_alt_was_released() {
    let replay = AltTabRecoveryPlan::create(false, false, false);
    assert_eq!(replay.len(), 4);
    assert_eq!(replay[0].key, SwitcherKey::Alt);
    assert_eq!(replay[0].transition, KeyTransition::Down);
    assert_eq!(replay[1].key, SwitcherKey::Tab);
    assert_eq!(replay[1].transition, KeyTransition::Down);
    assert_eq!(replay[2].key, SwitcherKey::Tab);
    assert_eq!(replay[2].transition, KeyTransition::Up);
    assert_eq!(replay[3].key, SwitcherKey::Alt);
    assert_eq!(replay[3].transition, KeyTransition::Up);
    assert!(replay.iter().all(|event| event.injected));
}

#[test]
fn recovery_plan_exposes_a_borrowed_slice_without_allocation() {
    let replay = AltTabRecoveryPlan::create(false, false, false);
    assert_eq!(replay.as_slice().len(), replay.len());
    assert_eq!(replay.iter().count(), replay.len());
    assert_eq!((&replay).into_iter().count(), replay.len());
    assert!(std::mem::size_of::<AltTabRecoveryPlan>() <= 128);
}

#[test]
fn releasing_one_shift_key_keeps_the_other_shift_key_down() {
    let mut state = KeyboardModifierState::new();
    state.create_input_with_modifier(
        SwitcherKey::Shift,
        KeyTransition::Down,
        false,
        false,
        KeyboardModifierKey::LeftShift,
    );
    state.create_input_with_modifier(
        SwitcherKey::Shift,
        KeyTransition::Down,
        false,
        false,
        KeyboardModifierKey::RightShift,
    );
    state.create_input_with_modifier(
        SwitcherKey::Shift,
        KeyTransition::Up,
        false,
        false,
        KeyboardModifierKey::LeftShift,
    );
    let tab = state.create_input_with_modifier(
        SwitcherKey::Tab,
        KeyTransition::Down,
        true,
        false,
        KeyboardModifierKey::None,
    );
    assert!(tab.shift);
    assert!(state.shift_down());
}

#[test]
fn reported_alt_context_recovers_a_missed_modifier_transition_until_alt_up() {
    let mut state = KeyboardModifierState::new();
    let tab = state.create_input_with_modifier(
        SwitcherKey::Tab,
        KeyTransition::Down,
        true,
        false,
        KeyboardModifierKey::None,
    );
    assert!(tab.alt);
    assert!(state.alt_down());
    state.create_input_with_modifier(
        SwitcherKey::Alt,
        KeyTransition::Up,
        true,
        false,
        KeyboardModifierKey::LeftAlt,
    );
    assert!(!state.alt_down());
}

#[test]
fn failed_reverse_open_replays_shift_when_it_was_released_before_dispatch() {
    let replay = AltTabRecoveryPlan::create(false, true, false);
    assert_eq!(replay.len(), 6);
    assert_eq!(replay[0].key, SwitcherKey::Alt);
    assert_eq!(replay[1].key, SwitcherKey::Shift);
    assert_eq!(replay[1].transition, KeyTransition::Down);
    assert_eq!(replay[2].key, SwitcherKey::Tab);
    assert_eq!(replay[4].key, SwitcherKey::Shift);
    assert_eq!(replay[4].transition, KeyTransition::Up);
    assert_eq!(replay[5].key, SwitcherKey::Alt);
}

#[test]
fn forward_recovery_temporarily_excludes_a_newer_shift_press() {
    let replay = AltTabRecoveryPlan::create(true, false, true);
    assert_eq!(replay.len(), 4);
    assert_eq!(replay[0].key, SwitcherKey::Shift);
    assert_eq!(replay[0].transition, KeyTransition::Up);
    assert_eq!(replay[1].key, SwitcherKey::Tab);
    assert_eq!(replay[2].key, SwitcherKey::Tab);
    assert_eq!(replay[3].key, SwitcherKey::Shift);
    assert_eq!(replay[3].transition, KeyTransition::Down);
}

#[test]
fn aborting_pending_admission_clears_only_the_failed_initial_gesture() {
    let mut state = KeyboardSuppressionState::new();
    let (consumed, token) =
        state.should_consume_with_admission(down(SwitcherKey::Tab, true, false));
    assert!(consumed);
    assert_ne!(token, 0);
    assert!(state.admission_pending());
    assert!(state.abort_pending_admission(token));
    assert!(!state.session_active_or_pending());
    assert!(!state.admission_pending());
    assert!(!state.should_consume(up(SwitcherKey::Tab, true)));
}

#[test]
fn aborting_pending_admission_does_not_disable_an_already_visible_session() {
    let mut state = KeyboardSuppressionState::new();
    let (_, token) = state.should_consume_with_admission(down(SwitcherKey::Tab, true, false));
    state.set_session_visible(true);
    assert!(!state.admission_pending());
    assert!(!state.abort_pending_admission(token));
    assert!(state.session_active_or_pending());
    assert!(state.should_consume(down(SwitcherKey::D1, false, false)));
}

#[test]
fn failed_tab_repeat_cannot_cancel_an_admission_already_queued_for_the_ui() {
    let mut state = KeyboardSuppressionState::new();
    let tab_down = down(SwitcherKey::Tab, true, false);
    let (_, original_token) = state.should_consume_with_admission(tab_down);
    let (_, repeat_token) = state.should_consume_with_admission(tab_down);
    assert_ne!(original_token, 0);
    assert_eq!(repeat_token, 0);
    assert!(!state.abort_pending_admission(repeat_token));
    assert!(state.session_active_or_pending());
    assert!(state.admission_pending());
    assert!(state.abort_pending_admission(original_token));
}

#[test]
fn dispatcher_failed_post_releases_the_bounded_keyboard_delivery_slot() {
    let mut dispatcher = DeferredKeyboardDispatcher::new(1).unwrap();
    let event = down(SwitcherKey::Tab, true, false);
    let ticket = dispatcher
        .try_dispatch(event)
        .expect("first slot is available");
    assert_eq!(dispatcher.pending_count(), 1);
    assert!(dispatcher.post_failed(ticket));
    assert_eq!(dispatcher.pending_count(), 0);
    assert!(dispatcher.try_dispatch(event).is_some());
}

#[test]
fn dispatcher_rejects_duplicate_completion_and_reuses_the_exact_slot() {
    let mut dispatcher = DeferredKeyboardDispatcher::new(1).unwrap();
    let first = dispatcher
        .try_dispatch(down(SwitcherKey::Tab, true, false))
        .unwrap();
    assert!(dispatcher.complete(first));
    assert!(!dispatcher.complete(first));
    let second = dispatcher
        .try_dispatch(down(SwitcherKey::Escape, false, false))
        .unwrap();
    assert_ne!(first, second);
    assert!(dispatcher.complete(second));
}

#[test]
fn dispatcher_keeps_critical_slot_when_ordinary_queue_is_full() {
    let mut dispatcher = DeferredKeyboardDispatcher::new(1).unwrap();
    let ordinary = dispatcher
        .try_dispatch(down(SwitcherKey::Tab, true, false))
        .unwrap();
    let critical = dispatcher
        .try_dispatch_critical(up(SwitcherKey::Alt, true))
        .unwrap();
    assert!(dispatcher
        .try_dispatch(down(SwitcherKey::Tab, true, false))
        .is_none());
    assert!(dispatcher
        .try_dispatch_critical(up(SwitcherKey::Alt, true))
        .is_none());
    assert_eq!(dispatcher.ordinary_pending(), 1);
    assert_eq!(dispatcher.critical_pending(), 1);
    assert!(dispatcher.complete(ordinary));
    assert!(dispatcher.complete(critical));
    assert_eq!(dispatcher.pending_count(), 0);
}

#[test]
fn stale_dispatch_ticket_is_discarded_but_releases_its_slot() {
    let mut dispatcher = DeferredKeyboardDispatcher::new(1).unwrap();
    let stale = dispatcher
        .try_dispatch(down(SwitcherKey::Tab, true, false))
        .unwrap();
    assert!(dispatcher.should_deliver(stale));
    dispatcher.invalidate_pending();
    assert!(!stale.is_current(dispatcher.generation()));
    assert!(!dispatcher.should_deliver(stale));
    assert!(dispatcher.complete(stale));
    let current = dispatcher
        .try_dispatch(down(SwitcherKey::Tab, true, true))
        .unwrap();
    assert!(current.is_current(dispatcher.generation()));
    assert!(dispatcher.post_failed(current));
}

#[test]
fn hook_path_state_has_fixed_copy_storage_without_lock_or_heap_state() {
    assert_eq!(std::mem::size_of::<KeyboardModifierState>(), 2);
    assert_eq!(std::mem::size_of::<KeyboardSuppressionState>(), 24);
    assert!(std::mem::size_of::<DispatchTicket>() <= 64);
}

#[test]
fn dispatcher_rejects_zero_or_unrepresentable_capacity() {
    assert!(DeferredKeyboardDispatcher::new(0).is_err());
    assert!(DeferredKeyboardDispatcher::new(65).is_err());
    assert_eq!(DeferredKeyboardDispatcher::default().capacity(), 64);
}
