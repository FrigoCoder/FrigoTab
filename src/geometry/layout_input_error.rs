/// Constructor failures corresponding to the original argument validation.
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
