use crate::geometry::screen_point::ScreenPoint;

/// The boundary between the deterministic switcher state machine and the
/// Win32 implementation. The implementation owns native windows, thumbnails,
/// drawing resources, and foreground-window calls.
#[allow(clippy::result_unit_err)]
pub trait SwitcherSessionPort {
    /// Attempts to construct and show a session. `Ok(0)` is equivalent to the
    /// `false` result with no candidates; a non-zero count opens a usable
    /// session. `Err(())` represents a native/session construction failure.
    fn try_open(&mut self) -> Result<usize, ()>;

    /// Selects a candidate by its zero-based index.
    fn select(&mut self, index: usize) -> Result<(), ()>;

    /// Clears the current selection when the pointer is outside every
    /// candidate.
    fn clear_selection(&mut self) -> Result<(), ()>;

    /// Returns the candidate under a screen point, or `None` when the point is
    /// not over a selectable candidate.
    fn hit_test(&mut self, point: ScreenPoint) -> Result<Option<usize>, ()>;

    /// Updates the thumbnail currently under the pointer independently of the
    /// keyboard selection. Native-style close buttons use this hover state.
    fn set_hovered(&mut self, index: Option<usize>) -> Result<(), ()>;

    /// Applies pointer hover and pointer selection as one visual transition.
    fn set_pointer_index(&mut self, index: Option<usize>) -> Result<(), ()>;

    /// Requests closure when the point is over an enabled close button.
    /// Returns `true` when the click belongs to that control, regardless of
    /// whether the target eventually accepts the asynchronous close request.
    fn try_close_at(&mut self, point: ScreenPoint) -> Result<bool, ()>;

    /// Checks an asynchronous close request and rebuilds the live preview
    /// graph once its source is no longer a switchable window. `None` means
    /// that no refresh is ready; `Some(count)` publishes the replacement.
    fn refresh_closed_applications(&mut self) -> Result<Option<usize>, ()>;

    /// Attempts to activate the currently selected candidate. `Ok(false)`
    /// leaves the session visible so the user can retry or cancel.
    fn try_activate_selected(&mut self) -> Result<bool, ()>;

    /// Hides the session and releases native resources. It is safe to call
    /// after a partial or failed open.
    fn close(&mut self);

    /// Recalculates the session for the current display topology and DPI.
    fn relayout(&mut self) -> Result<(), ()>;
}
