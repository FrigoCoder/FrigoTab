//! The collection of application previews owned by one switcher session.
//!
//! This is the native `ApplicationWindows` implementation. In
//! particular, enumeration order is preserved, layout failures only remove the
//! stale candidate that caused them, and the selection/visibility transitions
//! are propagated to every native preview in the same order as the original.

use windows_sys::Win32::Foundation::{HWND, RECT};

use super::application_window::ApplicationWindow;
use super::window_finder::WindowFinder;
use super::window_handle::WindowHandle;
use crate::geometry::Layout;
use crate::switcher::CloseButtonMode;

/// The live previews for one switcher owner window.
pub struct ApplicationWindows {
    windows: Vec<ApplicationWindow>,
    selected: Option<usize>,
    hovered: Option<usize>,
    visible: bool,
    disposed: bool,
}

impl ApplicationWindows {
    /// Build previews from one `WindowFinder` snapshot.
    ///
    /// Perform layout once, then try each candidate in
    /// `EnumWindows` order.  A candidate can disappear between enumeration,
    /// layout, and thumbnail registration; that candidate is skipped while
    /// already valid previews remain part of the session.
    pub fn new(owner: WindowHandle, finder: &WindowFinder) -> Self {
        Self::with_close_button_mode(owner, finder, CloseButtonMode::AlwaysVisible)
    }

    pub(crate) fn with_close_button_mode(
        owner: WindowHandle,
        finder: &WindowFinder,
        close_button_mode: CloseButtonMode,
    ) -> Self {
        Self::build(owner, finder, close_button_mode, true)
    }

    pub(crate) fn prepared_hidden(
        owner: WindowHandle,
        finder: &WindowFinder,
        close_button_mode: CloseButtonMode,
    ) -> Self {
        Self::build(owner, finder, close_button_mode, false)
    }

    fn build(
        owner: WindowHandle,
        finder: &WindowFinder,
        close_button_mode: CloseButtonMode,
        thumbnails_visible: bool,
    ) -> Self {
        let layout = Layout::new(&finder.windows);
        let mut windows = Vec::new();
        for application in finder.windows.iter().copied() {
            let Some(bounds) = layout.bounds.get(&application).copied() else {
                continue;
            };
            let native_bounds = RECT {
                left: bounds.x,
                top: bounds.y,
                right: bounds.right(),
                bottom: bounds.bottom(),
            };
            // Number a tile by the number of previews
            // already accepted, not by the original EnumWindows index.  A
            // stale candidate therefore cannot leave a gap in the shortcuts.
            let index = windows.len();
            match ApplicationWindow::with_close_button_mode_and_thumbnail_visibility(
                owner.raw(),
                application.raw(),
                index,
                native_bounds,
                close_button_mode,
                thumbnails_visible,
            ) {
                Ok(window) => windows.push(window),
                Err(error) => {
                    // Keep the candidate-independent failure policy. A stale
                    // HWND or one failed DWM/icon resource must not throw away
                    // previews which were already constructed.
                    debug_log(&format!("Skipping unavailable application window: {error}"));
                }
            }
        }

        Self {
            windows,
            selected: None,
            hovered: None,
            visible: false,
            disposed: false,
        }
    }

    pub fn count(&self) -> usize {
        self.windows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn windows(&self) -> &[ApplicationWindow] {
        &self.windows
    }

    /// Set the selected preview and propagate the visual state transition.
    pub fn select_by_index(&mut self, index: Option<usize>) -> Result<(), String> {
        if self.disposed {
            return Ok(());
        }
        if self.selected == index {
            return Ok(());
        }
        if let Some(old) = self.selected
            && let Some(window) = self.windows.get_mut(old)
        {
            window.set_selected(false)?;
        }
        self.selected = index.filter(|value| *value < self.windows.len());
        if let Some(new) = self.selected
            && let Some(window) = self.windows.get_mut(new)
        {
            window.set_selected(true)?;
        }
        Ok(())
    }

    /// Show or hide every layered application overlay. The DWM thumbnails are
    /// already prepared behind the hidden owner during construction.
    pub fn set_visible(&mut self, value: bool) {
        if self.disposed || self.visible == value {
            return;
        }
        self.visible = value;
        for window in &mut self.windows {
            window.set_session_visible(value);
        }
    }

    pub(crate) fn set_thumbnails_visible(&self, visible: bool) -> Result<(), i32> {
        if self.disposed {
            return Ok(());
        }
        for window in &self.windows {
            window.set_thumbnail_visible(visible)?;
        }
        Ok(())
    }

    /// Update the per-tile close-button mode and redraw current overlays.
    ///
    /// A redraw failure is isolated to the affected tile. The native preview
    /// collection remains usable, matching the best-effort behavior of the
    /// other visual state updates in this adapter.
    pub fn set_close_button_mode(&mut self, mode: CloseButtonMode) {
        if self.disposed {
            return;
        }
        for window in &mut self.windows {
            if let Err(error) = window.set_close_button_mode(mode) {
                debug_log(&format!("Could not redraw close button: {error}"));
            }
        }
    }

    /// Update the tile currently under the pointer. Only the previous and
    /// current tiles are redrawn when the index changes.
    pub fn set_hovered_index(&mut self, index: Option<usize>) -> Result<(), String> {
        if self.disposed {
            return Ok(());
        }
        let next = index.filter(|value| *value < self.windows.len());
        if self.hovered == next {
            return Ok(());
        }

        if let Some(old) = self.hovered
            && let Some(window) = self.windows.get_mut(old)
        {
            window.set_hovered(false)?;
        }
        self.hovered = next;
        if let Some(new) = self.hovered
            && let Some(window) = self.windows.get_mut(new)
        {
            window.set_hovered(true)?;
        }
        Ok(())
    }

    /// Apply pointer hover and pointer selection together so each affected
    /// layered tile is published at most once during a transition.
    pub fn set_pointer_index(&mut self, index: Option<usize>) -> Result<(), String> {
        if self.disposed {
            return Ok(());
        }
        let next = index.filter(|value| *value < self.windows.len());
        if self.selected == next && self.hovered == next {
            return Ok(());
        }
        self.selected = next;
        self.hovered = next;
        for (index, window) in self.windows.iter_mut().enumerate() {
            let active = Some(index) == next;
            window.set_interaction_state(active, active)?;
        }
        Ok(())
    }

    pub fn hovered_index(&self) -> Option<usize> {
        self.hovered
    }

    /// Return the source HWND of the close control under a screen point.
    ///
    /// This method has no side effects. The session adapter owns the close
    /// request and can therefore deduplicate it and schedule an asynchronous
    /// refresh after the target has actually disappeared.
    pub fn close_target_at(&self, x: i32, y: i32) -> Option<HWND> {
        if self.disposed {
            return None;
        }
        self.windows
            .iter()
            .find(|window| window.close_button_hit(x, y))
            .map(ApplicationWindow::application)
    }

    /// Return the first tile containing a screen point.
    pub fn hit_test(&self, x: i32, y: i32) -> Option<usize> {
        if self.disposed {
            return None;
        }
        self.windows.iter().position(|window| {
            let bounds = window.bounds();
            x >= bounds.left && x < bounds.right && y >= bounds.top && y < bounds.bottom
        })
    }

    pub fn try_activate_selected(&self) -> bool {
        if self.disposed {
            return false;
        }
        self.selected
            .and_then(|index| self.windows.get(index))
            .is_some_and(ApplicationWindow::try_activate)
    }

    /// Release all native preview windows.  This is intentionally idempotent
    /// and best effort, matching `ApplicationWindows.Dispose`.
    pub fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        if self.visible {
            for window in &mut self.windows {
                window.set_session_visible(false);
            }
        }
        self.visible = false;
        if let Some(index) = self.selected
            && let Some(window) = self.windows.get_mut(index)
        {
            let _ = window.set_selected(false);
        }
        self.selected = None;
        self.hovered = None;
        // Dropping every ApplicationWindow releases its child HWND, layered
        // bitmap, icon, and DWM thumbnail in the same collection order.
        self.windows.clear();
    }
}

impl Drop for ApplicationWindows {
    fn drop(&mut self) {
        self.dispose();
    }
}

fn debug_log(message: &str) {
    // Native debug output is intentionally used instead of introducing a
    // logging dependency for this tiny desktop process.
    let mut wide: Vec<u16> = message.encode_utf16().collect();
    wide.push(0);
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::OutputDebugStringW(wide.as_ptr());
    }
}
