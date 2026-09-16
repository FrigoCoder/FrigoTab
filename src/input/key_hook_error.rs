use std::fmt;

/// Errors returned while starting or shutting down the native hook.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum KeyHookError {
    InvalidOwner,
    ThreadStart,
    StartupTimeout,
    ThreadExited,
    AlreadyInstalled,
    Win32 { operation: &'static str, code: u32 },
}

impl fmt::Display for KeyHookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOwner => {
                formatter.write_str("keyboard hook owner HWND is null or invalid")
            }
            Self::ThreadStart => formatter.write_str("unable to start keyboard hook thread"),
            Self::StartupTimeout => formatter.write_str("keyboard hook startup timed out"),
            Self::ThreadExited => formatter.write_str("keyboard hook thread exited during startup"),
            Self::AlreadyInstalled => {
                formatter.write_str("only one FrigoTab keyboard hook may be installed")
            }
            Self::Win32 { operation, code } => {
                write!(formatter, "{operation} failed with Win32 error {code}")
            }
        }
    }
}

impl std::error::Error for KeyHookError {}
