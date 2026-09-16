use frigotab::session_window::SessionWindow;
use frigotab::switcher_application::SwitcherApplication;

pub(super) struct LiveSessionState {
    pub(super) controller: SwitcherApplication,
    pub(super) session: Option<SessionWindow>,
}
