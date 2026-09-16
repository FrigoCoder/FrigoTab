use super::layout_input_error::LayoutInputError;
use super::screen_rectangle::ScreenRectangle;

/// A window candidate and the monitor to which its layout belongs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutWindow {
    pub id: String,
    pub monitor_id: String,
    pub bounds: ScreenRectangle,
}

impl LayoutWindow {
    pub fn new(
        id: impl Into<String>,
        monitor_id: impl Into<String>,
        bounds: ScreenRectangle,
    ) -> Result<Self, LayoutInputError> {
        let id = id.into();
        if id.is_empty() {
            return Err(LayoutInputError::EmptyId);
        }
        let monitor_id = monitor_id.into();
        if monitor_id.is_empty() {
            return Err(LayoutInputError::EmptyMonitorId);
        }
        Ok(Self {
            id,
            monitor_id,
            bounds,
        })
    }
}
