use std::cell::RefCell;
use std::rc::Rc;

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{BLACKNESS, HDC, PaintDesktop, PatBlt};

use crate::desktop::shell_desktop_snapshot::ShellDesktopSnapshot;

use super::background_mode::BackgroundMode;
use super::background_state::BackgroundState;

/// A cloneable, UI-thread-only view of the owner window's paint state.
///
/// Win32 can synchronously deliver `WM_PAINT` from inside `UpdateWindow`.
/// Keeping this state in its own `Rc<RefCell<_>>` lets the WndProc repaint
/// without aliasing the mutable `App`/`SessionWindow` borrow which initiated
/// the native call.
#[derive(Clone)]
pub struct SessionPainter(Rc<RefCell<BackgroundState>>);

impl SessionPainter {
    pub(super) fn new(
        desktop_snapshot: Option<ShellDesktopSnapshot>,
        desktop_snapshot_bounds: RECT,
    ) -> Self {
        Self(Rc::new(RefCell::new(BackgroundState {
            mode: BackgroundMode::FullDesktop,
            desktop_snapshot,
            desktop_snapshot_bounds,
            owner_bounds: RECT::default(),
            disposed: false,
        })))
    }

    pub fn mode(&self) -> BackgroundMode {
        self.0.borrow().mode
    }

    pub(super) fn set_mode(&self, mode: BackgroundMode) {
        self.0.borrow_mut().mode = mode;
    }

    pub(super) fn has_snapshot_for(&self, bounds: RECT) -> bool {
        let state = self.0.borrow();
        state.desktop_snapshot.is_some() && same_rect(state.desktop_snapshot_bounds, bounds)
    }

    pub(super) fn set_owner_bounds(&self, bounds: RECT) {
        self.0.borrow_mut().owner_bounds = bounds;
    }

    pub(super) fn publish_snapshot(&self, snapshot: ShellDesktopSnapshot, bounds: RECT) {
        let mut state = self.0.borrow_mut();
        state.desktop_snapshot = Some(snapshot);
        state.desktop_snapshot_bounds = bounds;
    }

    pub(super) fn dispose(&self) {
        let mut state = self.0.borrow_mut();
        state.disposed = true;
        state.desktop_snapshot = None;
        state.desktop_snapshot_bounds = RECT::default();
    }

    /// Paint the configured background into an owner-window client DC.
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HDC is an opaque GDI handle.
    pub fn paint(&self, destination_dc: HDC, destination_bounds: RECT) {
        let Ok(state) = self.0.try_borrow() else {
            return;
        };
        if state.disposed
            || destination_bounds.right <= destination_bounds.left
            || destination_bounds.bottom <= destination_bounds.top
        {
            return;
        }
        match state.mode {
            BackgroundMode::FullDesktop => {
                if let Some(snapshot) = state
                    .desktop_snapshot
                    .as_ref()
                    .filter(|_| same_rect(state.desktop_snapshot_bounds, state.owner_bounds))
                {
                    snapshot.draw(destination_dc, destination_bounds);
                } else {
                    paint_black(destination_dc, destination_bounds);
                }
            }
            BackgroundMode::ImageOnly => {
                if unsafe { PaintDesktop(destination_dc) } == 0 {
                    paint_black(destination_dc, destination_bounds);
                }
            }
            BackgroundMode::Black => paint_black(destination_dc, destination_bounds),
        }
    }
}

fn same_rect(left: RECT, right: RECT) -> bool {
    left.left == right.left
        && left.top == right.top
        && left.right == right.right
        && left.bottom == right.bottom
}

fn paint_black(destination_dc: HDC, destination_bounds: RECT) {
    if destination_dc.is_null()
        || destination_bounds.right <= destination_bounds.left
        || destination_bounds.bottom <= destination_bounds.top
    {
        return;
    }
    unsafe {
        PatBlt(
            destination_dc,
            destination_bounds.left,
            destination_bounds.top,
            destination_bounds.right - destination_bounds.left,
            destination_bounds.bottom - destination_bounds.top,
            BLACKNESS,
        );
    }
}
