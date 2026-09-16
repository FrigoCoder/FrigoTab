use super::layout_input_error::LayoutInputError;
use super::screen_rectangle::ScreenRectangle;

/// Display bounds and usable working area for one monitor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutMonitor {
    pub id: String,
    pub bounds: ScreenRectangle,
    pub working_area: ScreenRectangle,
}

impl LayoutMonitor {
    pub fn new(
        id: impl Into<String>,
        bounds: ScreenRectangle,
        working_area: ScreenRectangle,
    ) -> Result<Self, LayoutInputError> {
        let id = id.into();
        if id.is_empty() {
            return Err(LayoutInputError::EmptyId);
        }
        Ok(Self {
            id,
            bounds,
            working_area,
        })
    }
}
