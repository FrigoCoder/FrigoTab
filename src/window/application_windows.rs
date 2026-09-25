//! The collection of application previews owned by one switcher session.
//!
//! This is the native `ApplicationWindows` implementation. In
//! particular, enumeration order is preserved, layout failures only remove the
//! stale candidate that caused them, and the selection/visibility transitions
//! are propagated to every native preview in the same order as the original.

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

use super::application_window::ApplicationWindow;
use super::window_finder::WindowFinder;
use super::window_handle::WindowHandle;
use crate::geometry::Layout;

/// The live previews for one switcher owner window.
pub struct ApplicationWindows {
    windows: Vec<ApplicationWindow>,
    selected: Option<usize>,
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
        Self::with_close_buttons(owner, finder, true)
    }

    pub(crate) fn with_close_buttons(
        owner: WindowHandle,
        finder: &WindowFinder,
        close_buttons_visible: bool,
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
            match ApplicationWindow::with_close_buttons(
                owner.raw(),
                application.raw(),
                index,
                native_bounds,
                close_buttons_visible,
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

    /// Show or hide the per-tile close buttons and redraw current overlays.
    ///
    /// A redraw failure is isolated to the affected tile. The native preview
    /// collection remains usable, matching the best-effort behavior of the
    /// other visual state updates in this adapter.
    pub fn set_close_buttons_visible(&mut self, value: bool) {
        if self.disposed {
            return;
        }
        for window in &mut self.windows {
            if let Err(error) = window.set_close_buttons_visible(value) {
                debug_log(&format!("Could not redraw close button: {error}"));
            }
        }
    }

    /// Request closure of the tile under a screen point.
    ///
    /// A point in a close-button region is consumed even when the target
    /// rejects the asynchronous `WM_CLOSE` post. The collection is left
    /// unchanged: applications may display a confirmation dialog or otherwise
    /// defer/veto closure, and stale-source handling remains the same as for
    /// windows closed outside the switcher.
    pub fn try_close_at(&self, x: i32, y: i32) -> bool {
        if self.disposed {
            return false;
        }
        let Some(window) = self
            .windows
            .iter()
            .find(|window| window.close_button_hit(x, y))
        else {
            return false;
        };

        unsafe {
            let _ = PostMessageW(window.application(), WM_CLOSE, 0, 0);
        }
        true
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
