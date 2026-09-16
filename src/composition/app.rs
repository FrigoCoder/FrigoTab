use frigotab::geometry::ScreenPoint;
use frigotab::input::{KeyHandling, KeyHook, KeyTransition, KeyboardInput, SwitcherKey};
use frigotab::switcher::{SessionWindow, SwitcherApplication, SwitcherState};
use frigotab::tray::SysTrayIcon;
use windows_sys::Win32::Foundation::WPARAM;

/// The one UI-thread-owned object graph corresponding to `Program.Run`.
/// `SessionWindow` is kept separate from the deterministic controller just as
/// `SessionWindow` is separate from `SwitcherApplication` in the original
/// application.
pub(crate) struct App {
    pub(crate) controller: SwitcherApplication,
    pub(crate) session: Option<SessionWindow>,
    pub(crate) hook: Option<KeyHook>,
    pub(crate) tray: Option<SysTrayIcon>,
    reported_session_visibility: bool,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            controller: SwitcherApplication::new(),
            session: None,
            hook: None,
            tray: None,
            reported_session_visibility: false,
        }
    }

    pub(crate) fn handle_keyboard(&mut self, input: KeyboardInput) -> KeyHandling {
        let handling = if let Some(session) = self.session.as_mut() {
            self.controller.handle_keyboard(session, input)
        } else {
            KeyHandling::PassThrough
        };
        self.sync_session_visibility();
        handling
    }

    pub(crate) fn handle_mouse_move(&mut self, point: ScreenPoint) {
        if let Some(session) = self.session.as_mut() {
            self.controller.handle_mouse_move(session, point);
            self.sync_session_visibility();
        }
    }

    pub(crate) fn handle_mouse_click(&mut self, point: ScreenPoint) {
        if let Some(session) = self.session.as_mut() {
            self.controller.handle_mouse_click(session, point);
            self.sync_session_visibility();
        }
    }

    pub(crate) fn begin_session(&mut self) {
        let input = KeyboardInput::new(SwitcherKey::Tab, KeyTransition::Down, true, false, false);
        let _ = self.handle_keyboard(input);
    }

    pub(crate) fn dispatch_hook_message(&mut self, wparam: WPARAM) {
        let Some(hook) = self.hook.as_ref() else {
            return;
        };
        // `dispatch_ui_message` invokes the handler synchronously on this UI
        // thread. A raw pointer avoids borrowing `self.hook` across the
        // closure while keeping the KeyHook alive for the entire call.
        let hook = hook as *const KeyHook;
        unsafe {
            (*hook).dispatch_ui_message(wparam, |input| self.handle_keyboard(input));
        }
    }

    pub(crate) fn sync_session_visibility(&mut self) {
        let visible = self.controller.state() == SwitcherState::Visible;
        if visible != self.reported_session_visibility {
            if let Some(hook) = self.hook.as_ref() {
                hook.set_session_visible(visible);
            }
            self.reported_session_visibility = visible;
        }
    }

    pub(crate) fn interrupt(&mut self) {
        if let Some(session) = self.session.as_mut() {
            self.controller.interrupt(session);
        }
        if let Some(hook) = self.hook.as_ref() {
            hook.reset_input_state();
        }
        self.sync_session_visibility();
    }

    pub(crate) fn close_for_shutdown(&mut self) {
        if let Some(session) = self.session.as_mut() {
            // SessionForm.Dispose marks itself disposed before controller.Close,
            // which prevents CloseSessionResources from queuing a final shell
            // capture while the application is shutting down.
            session.dispose();
            self.controller.close(session);
        }
        self.sync_session_visibility();
        if let Some(hook) = self.hook.take() {
            hook.drain_ui_messages();
            drop(hook);
        }
        // Remove the notification-area registration while the owner HWND is
        // still valid. This prevents a failed NIM_DELETE from leaving a
        // stale icon after the owner is destroyed.
        drop(self.tray.take());
    }

    pub(crate) fn relayout(&mut self) {
        let was_visible = self.controller.state() == SwitcherState::Visible;
        if let Some(session) = self.session.as_mut() {
            self.controller.relayout(session);
            if was_visible
                && self.controller.state() != SwitcherState::Visible
                && let Some(hook) = self.hook.as_ref()
            {
                hook.reset_input_state();
            }
            session.queue_desktop_snapshot_refresh_current();
        }
        self.sync_session_visibility();
    }

    pub(crate) fn publish_snapshot(&mut self) {
        if let Some(session) = self.session.as_mut() {
            session.publish_desktop_snapshot();
        }
    }
}
