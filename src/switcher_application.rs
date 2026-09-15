use crate::key_handling::KeyHandling;
use crate::keyboard_input::{KeyboardInput, SwitcherKey};
use crate::screen_point::ScreenPoint;
use crate::switcher_state::SwitcherState;

/// The boundary between the deterministic switcher state machine and the
/// Win32 implementation.  The implementation owns native windows, thumbnails,
/// drawing resources, and foreground-window calls.
#[allow(clippy::result_unit_err)]
pub trait SwitcherSessionPort {
    /// Attempts to construct and show a session.  `Ok(0)` is equivalent to the
    /// `false` result with no candidates; a non-zero count opens a usable
    /// session. `Err(())` represents a native/session construction failure.
    fn try_open(&mut self) -> Result<usize, ()>;

    /// Selects a candidate by its zero-based index.
    fn select(&mut self, index: usize) -> Result<(), ()>;

    /// Clears the current selection when the pointer is outside every
    /// candidate.
    fn clear_selection(&mut self) -> Result<(), ()>;

    /// Returns the candidate under a screen point, or `None` when the point is
    /// not over a selectable candidate.
    fn hit_test(&mut self, point: ScreenPoint) -> Result<Option<usize>, ()>;

    /// Attempts to activate the currently selected candidate.  `Ok(false)`
    /// leaves the session visible so the user can retry or cancel.
    fn try_activate_selected(&mut self) -> Result<bool, ()>;

    /// Hides the session and releases native resources.  It is safe to call
    /// after a partial or failed open.
    fn close(&mut self);

    /// Recalculates the session for the current display topology and DPI.
    fn relayout(&mut self) -> Result<(), ()>;
}

/// Deterministic application boundary for an Alt-Tab replacement.
///
/// This type deliberately knows nothing about HWNDs, DWM, or drawing. Those
/// operations are supplied by `SwitcherSessionPort` so the production Win32
/// adapter and the acceptance executable use the same interaction contract.
#[derive(Default)]
pub struct SwitcherApplication {
    consumed_keys: u32,
    state: SwitcherState,
    candidate_count: usize,
    selected_index: Option<usize>,
}

impl SwitcherApplication {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> SwitcherState {
        self.state
    }

    pub fn candidate_count(&self) -> usize {
        self.candidate_count
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Handles a normalized keyboard event and returns whether the global hook
    /// should consume it. The first Alt+Tab is consumed only after opening
    /// succeeds.
    pub fn handle_keyboard<P: SwitcherSessionPort>(
        &mut self,
        port: &mut P,
        input: KeyboardInput,
    ) -> KeyHandling {
        if input.injected {
            return KeyHandling::PassThrough;
        }

        // A global hook consumes both halves of a gesture. Several actions
        // close the session on key-down, so this ledger survives reset_state
        // until the physical key is released.
        let key_bit = input.key.bit();
        if input.is_up() && self.consumed_keys & key_bit != 0 {
            self.consumed_keys &= !key_bit;
            return KeyHandling::Consume;
        }

        let handling = if self.state == SwitcherState::Idle {
            self.handle_idle_keyboard(port, input)
        } else {
            self.handle_visible_keyboard(port, input)
        };
        if input.is_down() && handling == KeyHandling::Consume {
            // Auto-repeat produces multiple downs followed by one up; one bit
            // models that physical-key lifetime without requiring a count.
            self.consumed_keys |= key_bit;
        }
        handling
    }

    /// Updates selection from a pointer move over the session surface.
    pub fn handle_mouse_move<P: SwitcherSessionPort>(&mut self, port: &mut P, point: ScreenPoint) {
        if self.state != SwitcherState::Visible {
            return;
        }

        let index = match port.hit_test(point) {
            Ok(index) => index,
            Err(()) => {
                self.close(port);
                return;
            }
        };
        match index {
            Some(index) if self.is_valid_index(index) => {
                self.try_select(port, index);
            }
            Some(_) => self.close(port),
            None => self.try_clear_selection(port),
        }
    }

    /// Selects and activates the candidate under a pointer click.
    pub fn handle_mouse_click<P: SwitcherSessionPort>(&mut self, port: &mut P, point: ScreenPoint) {
        if self.state != SwitcherState::Visible {
            return;
        }

        let index = match port.hit_test(point) {
            Ok(index) => index,
            Err(()) => {
                self.close(port);
                return;
            }
        };
        let Some(index) = index else {
            self.try_clear_selection(port);
            return;
        };
        if !self.is_valid_index(index) {
            self.close(port);
            return;
        }
        if self.try_select(port, index) {
            self.try_commit_selection(port);
        }
    }

    /// Closes the active session. It is safe to call while idle.
    pub fn close<P: SwitcherSessionPort>(&mut self, port: &mut P) {
        // Native cleanup is best effort inside the real session adapter.
        port.close();
        self.reset_state();
    }

    /// Handles workstation lock, desktop interruption, or another event that
    /// makes the current overlay unusable.
    pub fn interrupt<P: SwitcherSessionPort>(&mut self, port: &mut P) {
        self.close(port);
        self.consumed_keys = 0;
    }

    /// Recalculates the visible session after a display/DPI topology change.
    /// A failed relayout closes the session and clears its consumed ledger.
    pub fn relayout<P: SwitcherSessionPort>(&mut self, port: &mut P) {
        if self.state != SwitcherState::Visible {
            return;
        }
        if port.relayout().is_err() {
            self.interrupt(port);
        }
    }

    fn handle_idle_keyboard<P: SwitcherSessionPort>(
        &mut self,
        port: &mut P,
        input: KeyboardInput,
    ) -> KeyHandling {
        if !input.is_down() || input.key != SwitcherKey::Tab || !input.alt {
            return KeyHandling::PassThrough;
        }
        if self.try_open(port) {
            KeyHandling::Consume
        } else {
            KeyHandling::PassThrough
        }
    }

    fn handle_visible_keyboard<P: SwitcherSessionPort>(
        &mut self,
        port: &mut P,
        input: KeyboardInput,
    ) -> KeyHandling {
        if input.is_down() && input.key == SwitcherKey::Tab && input.alt {
            self.move_selection(port, if input.shift { -1 } else { 1 });
            return KeyHandling::Consume;
        }
        if input.is_down() && input.key == SwitcherKey::Escape {
            self.close(port);
            return KeyHandling::Consume;
        }
        if input.is_down() && input.key == SwitcherKey::F4 && input.alt {
            self.close(port);
            return KeyHandling::Consume;
        }
        if input.is_down()
            && let Some(index) = digit_index(input.key)
        {
            // An out-of-range digit is still consumed while visible, but
            // it must not clear the current selection.
            if self.is_valid_index(index) {
                self.try_select(port, index);
                self.try_commit_selection(port);
            }
            return KeyHandling::Consume;
        }
        if input.is_up() && input.key == SwitcherKey::Alt {
            // FrigoTab is sticky: releasing Alt does not choose a target; a
            // number or pointer click does that. Passing the release through
            // balances the physical Alt-down that opened the session.
            return KeyHandling::PassThrough;
        }
        KeyHandling::PassThrough
    }

    fn try_open<P: SwitcherSessionPort>(&mut self, port: &mut P) -> bool {
        let count = match port.try_open() {
            Ok(count) if count > 0 => count,
            Ok(_) | Err(()) => {
                self.close(port);
                return false;
            }
        };

        // Keep state private until the initial selection succeeds; a partially
        // constructed native session is never published.
        if port.select(0).is_err() {
            self.close(port);
            return false;
        }
        self.state = SwitcherState::Visible;
        self.candidate_count = count;
        self.selected_index = Some(0);
        true
    }

    fn move_selection<P: SwitcherSessionPort>(&mut self, port: &mut P, direction: i32) {
        if self.candidate_count == 0 {
            return;
        }
        // Pointer movement outside every tile clears selection. Keyboard
        // traversal takes control again from the first or last candidate.
        let Some(selected) = self.selected_index else {
            self.try_select(
                port,
                if direction < 0 {
                    self.candidate_count - 1
                } else {
                    0
                },
            );
            return;
        };
        let next = if direction < 0 {
            if selected == 0 {
                self.candidate_count - 1
            } else {
                selected - 1
            }
        } else if selected + 1 >= self.candidate_count {
            0
        } else {
            selected + 1
        };
        self.try_select(port, next);
    }

    fn try_select<P: SwitcherSessionPort>(&mut self, port: &mut P, index: usize) -> bool {
        if self.state != SwitcherState::Visible || !self.is_valid_index(index) {
            return false;
        }
        if port.select(index).is_err() {
            self.close(port);
            return false;
        }
        self.selected_index = Some(index);
        true
    }

    fn try_clear_selection<P: SwitcherSessionPort>(&mut self, port: &mut P) {
        if port.clear_selection().is_err() {
            self.close(port);
            return;
        }
        self.selected_index = None;
    }

    fn try_commit_selection<P: SwitcherSessionPort>(&mut self, port: &mut P) {
        if self.state != SwitcherState::Visible || self.selected_index.is_none() {
            return;
        }
        // An activation exception is equivalent to failed activation: leave
        // the session available for retry/cancel.
        let Ok(activated) = port.try_activate_selected() else {
            return;
        };
        if activated {
            self.close(port);
        }
    }

    fn is_valid_index(&self, index: usize) -> bool {
        index < self.candidate_count
    }

    fn reset_state(&mut self) {
        self.state = SwitcherState::Idle;
        self.candidate_count = 0;
        self.selected_index = None;
    }
}

fn digit_index(key: SwitcherKey) -> Option<usize> {
    Some(match key {
        SwitcherKey::D1 | SwitcherKey::NumPad1 => 0,
        SwitcherKey::D2 | SwitcherKey::NumPad2 => 1,
        SwitcherKey::D3 | SwitcherKey::NumPad3 => 2,
        SwitcherKey::D4 | SwitcherKey::NumPad4 => 3,
        SwitcherKey::D5 | SwitcherKey::NumPad5 => 4,
        SwitcherKey::D6 | SwitcherKey::NumPad6 => 5,
        SwitcherKey::D7 | SwitcherKey::NumPad7 => 6,
        SwitcherKey::D8 | SwitcherKey::NumPad8 => 7,
        SwitcherKey::D9 | SwitcherKey::NumPad9 => 8,
        _ => return None,
    })
}
