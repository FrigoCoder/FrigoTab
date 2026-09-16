use std::{cell::RefCell, rc::Weak};

use windows_sys::Win32::UI::WindowsAndMessaging::HICON;

use super::icon_state::IconState;

/// Non-owning icon state used by the asynchronous Changed callback.  A weak
/// handle prevents the callback from keeping a disposed `WindowIcon` alive.
pub struct WindowIconWeak {
    pub(super) state: Weak<RefCell<IconState>>,
}

impl WindowIconWeak {
    pub fn with_icon<R>(&self, f: impl FnOnce(HICON, i32, i32) -> R) -> Option<R> {
        let state_arc = self.state.upgrade()?;
        let state = state_arc.borrow();
        let icon = state.icon.as_ref()?;
        (!state.disposed).then(|| f(icon.raw(), state.width, state.height))
    }
}
