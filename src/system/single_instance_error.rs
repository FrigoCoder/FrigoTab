//! Errors returned while probing or acquiring the application mutex.

/// Errors which prevent Windows from creating or probing the mutex.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SingleInstanceError {
    Create(u32),
    Wait(u32),
}

impl std::fmt::Display for SingleInstanceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create(error) => write!(formatter, "CreateMutexW failed (Win32 error {error})"),
            Self::Wait(error) => write!(
                formatter,
                "WaitForSingleObject failed (Win32 error {error})"
            ),
        }
    }
}

impl std::error::Error for SingleInstanceError {}
