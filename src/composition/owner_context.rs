use std::cell::RefCell;

use frigotab::switcher::SessionPainter;

use super::App;

/// Stable userdata for the owner HWND. The painter deliberately lives beside,
/// rather than inside, the dynamically borrowed application state so a
/// synchronous WM_PAINT can safely repaint during native session operations.
pub(crate) struct OwnerContext {
    pub(crate) app: RefCell<App>,
    pub(crate) painter: Option<SessionPainter>,
}

impl OwnerContext {
    pub(crate) fn new() -> Self {
        Self {
            app: RefCell::new(App::new()),
            painter: None,
        }
    }
}
