//! The collection of application previews owned by one switcher session.
//!
//! This is the native `ApplicationWindows` implementation. In
//! particular, enumeration order is preserved, layout failures only remove the
//! stale candidate that caused them, and the selection/visibility transitions
//! are propagated to every native preview in the same order as the original.

use windows_sys::Win32::Foundation::RECT;

use crate::application_window::ApplicationWindow;
use crate::layout::Layout;
use crate::window_finder::WindowFinder;
use crate::window_handle::WindowHandle;

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
            match ApplicationWindow::new(owner.raw(), application.raw(), index, native_bounds) {
                Ok(window) => windows.push(window),
                Err(error) => {
                    // Keep the candidate-independent failure policy. A
                    // stale HWND or one failed DWM/icon resource must not
                    // throw away previews which were already constructed.
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

    /// Show or hide every DWM preview, preserving the original event
    pub fn set_visible(&mut self, value: bool) -> Result<(), String> {
        if self.disposed || self.visible == value {
            return Ok(());
        }
        self.visible = value;
        for window in &mut self.windows {
            window.set_session_visible(value)?;
        }
        Ok(())
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
                let _ = window.set_session_visible(false);
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
