use std::collections::HashSet;

use crate::{KeyboardInput, ScreenPoint};

/// The decision returned to a keyboard-hook adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyHandling {
    PassThrough,
    Consume,
}

/// User-visible lifecycle states of a switcher session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwitcherState {
    Idle,
    Visible,
}

/// Error boundary between the deterministic policy and a platform adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortError {
    message: String,
}

impl PortError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PortError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PortError {}

/// Platform boundary for a switcher session.  Implementations own native
/// windows, thumbnails, drawing resources, and foreground-window calls.
pub trait SwitcherSessionPort {
    /// Returns the number of selectable candidates made available.  Zero is
    /// treated as an unusable session even when construction succeeds.
    fn try_open(&mut self) -> Result<usize, PortError>;
    fn select(&mut self, index: usize) -> Result<(), PortError>;
    fn clear_selection(&mut self) -> Result<(), PortError>;
    fn hit_test(&mut self, point: ScreenPoint) -> Result<Option<usize>, PortError>;
    /// A false result leaves the session visible so the user can retry/cancel.
    fn try_activate_selected(&mut self) -> Result<bool, PortError>;
    /// Cleanup is best effort from the application boundary.
    fn close(&mut self) -> Result<(), PortError>;
    fn relayout(&mut self) -> Result<(), PortError>;
}

/// Deterministic application boundary for an Alt-Tab replacement.
pub struct SwitcherApplication<P: SwitcherSessionPort> {
    port: P,
    consumed_keys: HashSet<crate::SwitcherKey>,
    state: SwitcherState,
    candidate_count: usize,
    selected_index: Option<usize>,
}

impl<P: SwitcherSessionPort> SwitcherApplication<P> {
    pub fn new(port: P) -> Self {
        Self {
            port,
            consumed_keys: HashSet::new(),
            state: SwitcherState::Idle,
            candidate_count: 0,
            selected_index: None,
        }
    }

    pub fn port(&self) -> &P {
        &self.port
    }

    pub fn port_mut(&mut self) -> &mut P {
        &mut self.port
    }

    pub const fn state(&self) -> SwitcherState {
        self.state
    }

    pub const fn candidate_count(&self) -> usize {
        self.candidate_count
    }

    pub const fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Handles a normalized keyboard event.  A global hook must consume both
    /// halves of a gesture, including releases that arrive after a key-down
    /// action closed the visible session.
    pub fn handle_keyboard(&mut self, input: KeyboardInput) -> KeyHandling {
        if input.injected {
            return KeyHandling::PassThrough;
        }

        if input.is_up() && self.consumed_keys.remove(&input.key) {
            return KeyHandling::Consume;
        }

        let handling = if self.state == SwitcherState::Idle {
            self.handle_idle_keyboard(input)
        } else {
            self.handle_visible_keyboard(input)
        };
        if input.is_down() && handling == KeyHandling::Consume {
            self.consumed_keys.insert(input.key);
        }
        handling
    }

    pub fn handle_mouse_move(&mut self, point: ScreenPoint) {
        if self.state != SwitcherState::Visible {
            return;
        }
        let index = match self.port.hit_test(point) {
            Ok(index) => index,
            Err(_) => {
                self.close_after_port_failure();
                return;
            }
        };
        match index {
            Some(index) if self.is_valid_index(index) => {
                self.try_select(index);
            }
            Some(_) => self.close_after_port_failure(),
            None => self.try_clear_selection(),
        }
    }

    pub fn handle_mouse_click(&mut self, point: ScreenPoint) {
        if self.state != SwitcherState::Visible {
            return;
        }
        let index = match self.port.hit_test(point) {
            Ok(index) => index,
            Err(_) => {
                self.close_after_port_failure();
                return;
            }
        };
        let Some(index) = index else {
            self.try_clear_selection();
            return;
        };
        if !self.is_valid_index(index) {
            self.close_after_port_failure();
            return;
        }
        if self.try_select(index) {
            self.try_commit_selection();
        }
    }

    /// Closes the active session; safe to call while idle.  Consumed-key
    /// entries deliberately survive until their physical releases arrive.
    pub fn close(&mut self) {
        let _ = self.port.close();
        self.reset_state();
    }

    pub fn interrupt(&mut self) {
        self.close();
        self.consumed_keys.clear();
    }

    pub fn relayout(&mut self) {
        if self.state != SwitcherState::Visible {
            return;
        }
        if self.port.relayout().is_err() {
            self.interrupt();
        }
    }

    fn handle_idle_keyboard(&mut self, input: KeyboardInput) -> KeyHandling {
        if input.is_down() && input.key == crate::SwitcherKey::Tab && input.alt {
            if self.try_open() {
                KeyHandling::Consume
            } else {
                KeyHandling::PassThrough
            }
        } else {
            KeyHandling::PassThrough
        }
    }

    fn handle_visible_keyboard(&mut self, input: KeyboardInput) -> KeyHandling {
        if input.is_down() && input.key == crate::SwitcherKey::Tab && input.alt {
            self.move_selection(if input.shift { -1 } else { 1 });
            return KeyHandling::Consume;
        }
        if input.is_down() && input.key == crate::SwitcherKey::Escape {
            self.close();
            return KeyHandling::Consume;
        }
        if input.is_down() && input.key == crate::SwitcherKey::F4 && input.alt {
            self.close();
            return KeyHandling::Consume;
        }
        if input.is_down() {
            if let Some(index) = digit_index(input.key) {
                if self.is_valid_index(index) {
                    self.try_select(index);
                    self.try_commit_selection();
                }
                return KeyHandling::Consume;
            }
        }
        // Releasing the physical Alt which admitted the session remains
        // pass-through so the foreground application is not left stuck.
        KeyHandling::PassThrough
    }

    fn try_open(&mut self) -> bool {
        let count = match self.port.try_open() {
            Ok(count) if count > 0 => count,
            _ => {
                self.close_after_port_failure();
                return false;
            }
        };
        if self.port.select(0).is_err() {
            self.close_after_port_failure();
            return false;
        }
        self.state = SwitcherState::Visible;
        self.candidate_count = count;
        self.selected_index = Some(0);
        true
    }

    fn move_selection(&mut self, direction: isize) {
        if self.candidate_count == 0 {
            return;
        }
        let index = match self.selected_index {
            Some(index) => index as isize + direction,
            None if direction < 0 => self.candidate_count as isize - 1,
            None => 0,
        };
        let index = if index < 0 {
            self.candidate_count - 1
        } else if index >= self.candidate_count as isize {
            0
        } else {
            index as usize
        };
        self.try_select(index);
    }

    fn try_select(&mut self, index: usize) -> bool {
        if self.state != SwitcherState::Visible || !self.is_valid_index(index) {
            return false;
        }
        if self.port.select(index).is_err() {
            self.close_after_port_failure();
            return false;
        }
        self.selected_index = Some(index);
        true
    }

    fn try_clear_selection(&mut self) {
        if self.port.clear_selection().is_err() {
            self.close_after_port_failure();
        } else {
            self.selected_index = None;
        }
    }

    fn try_commit_selection(&mut self) {
        if self.state != SwitcherState::Visible || self.selected_index.is_none() {
            return;
        }
        if matches!(self.port.try_activate_selected(), Ok(true)) {
            self.close();
        }
    }

    fn close_after_port_failure(&mut self) {
        let _ = self.port.close();
        self.reset_state();
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

fn digit_index(key: crate::SwitcherKey) -> Option<usize> {
    Some(match key {
        crate::SwitcherKey::D1 | crate::SwitcherKey::NumPad1 => 0,
        crate::SwitcherKey::D2 | crate::SwitcherKey::NumPad2 => 1,
        crate::SwitcherKey::D3 | crate::SwitcherKey::NumPad3 => 2,
        crate::SwitcherKey::D4 | crate::SwitcherKey::NumPad4 => 3,
        crate::SwitcherKey::D5 | crate::SwitcherKey::NumPad5 => 4,
        crate::SwitcherKey::D6 | crate::SwitcherKey::NumPad6 => 5,
        crate::SwitcherKey::D7 | crate::SwitcherKey::NumPad7 => 6,
        crate::SwitcherKey::D8 | crate::SwitcherKey::NumPad8 => 7,
        crate::SwitcherKey::D9 | crate::SwitcherKey::NumPad9 => 8,
        _ => return None,
    })
}
