use crate::ScreenRectangle;

/// Error returned when a layout identity is empty.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutInputError {
    EmptyId,
    EmptyMonitorId,
}

impl std::fmt::Display for LayoutInputError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => formatter.write_str("a layout identity needs a non-empty id"),
            Self::EmptyMonitorId => {
                formatter.write_str("a layout window needs a non-empty monitor id")
            }
        }
    }
}

impl std::error::Error for LayoutInputError {}

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
