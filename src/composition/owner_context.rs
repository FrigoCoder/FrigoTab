use std::cell::RefCell;
use std::collections::VecDeque;

use windows_sys::Win32::Foundation::WPARAM;

use frigotab::switcher::SessionPainter;

use super::App;

/// Stable userdata for the owner HWND. The painter deliberately lives beside,
/// rather than inside, the dynamically borrowed application state so a
/// synchronous WM_PAINT can safely repaint during native session operations.
pub(crate) struct OwnerContext {
    pub(crate) app: RefCell<App>,
    pub(crate) painter: Option<SessionPainter>,
    pub(crate) pending_hook_inputs: RefCell<VecDeque<WPARAM>>,
}

impl OwnerContext {
    pub(crate) fn new() -> Self {
        Self {
            app: RefCell::new(App::new()),
            painter: None,
            pending_hook_inputs: RefCell::new(VecDeque::new()),
        }
    }
}
