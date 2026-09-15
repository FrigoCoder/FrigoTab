//! The per-user mutex used by the tray application.
//!
//! This is the native equivalent of `SingleInstanceGuard`: construction does
//! not claim the mutex, a zero-time wait claims it (including an abandoned
//! mutex), and the claim is held until the guard is dropped.

use std::{ptr::NonNull, ptr::null};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INVALID_PARAMETER, GetLastError, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

pub const APPLICATION_MUTEX_NAME: &str = "Local\\FrigoTab";

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

/// Owns one successful wait on the application mutex.
pub struct SingleInstanceGuard {
    mutex: OwnedHandle,
}

struct OwnedHandle(NonNull<std::ffi::c_void>);

impl OwnedHandle {
    fn new(handle: HANDLE) -> Option<Self> {
        NonNull::new(handle).map(Self)
    }

    fn raw(&self) -> HANDLE {
        self.0.as_ptr()
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.raw());
        }
    }
}

impl SingleInstanceGuard {
    /// Try to acquire a named per-user mutex.
    ///
    /// `Ok(None)` means another process currently owns the mutex.  An
    /// abandoned mutex is treated as acquired, just like
    /// the abandoned-mutex condition in the original implementation.
    pub fn try_acquire(name: &str) -> Result<Option<Self>, SingleInstanceError> {
        if name.trim().is_empty() || name.contains('\0') {
            // Keep the invalid-name path explicit without making the native
            // layer panic.  Callers normally pass APPLICATION_MUTEX_NAME.
            return Err(SingleInstanceError::Create(ERROR_INVALID_PARAMETER));
        }

        let name = wide(name);
        let Some(mutex) = OwnedHandle::new(unsafe { CreateMutexW(null(), 0, name.as_ptr()) })
        else {
            return Err(SingleInstanceError::Create(unsafe { GetLastError() }));
        };

        let wait = unsafe { WaitForSingleObject(mutex.raw(), 0) };
        if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {
            return Ok(Some(Self { mutex }));
        }

        if wait == WAIT_TIMEOUT {
            return Ok(None);
        }

        let error = unsafe { GetLastError() };
        Err(SingleInstanceError::Wait(error))
    }

    /// Convenience form using the application's canonical mutex name.
    pub fn acquire() -> Result<Option<Self>, SingleInstanceError> {
        Self::try_acquire(APPLICATION_MUTEX_NAME)
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        // ReleaseMutex can fail only if exceptional teardown has already lost
        // ownership; OwnedHandle still closes the process handle afterward.
        unsafe {
            let _ = ReleaseMutex(self.mutex.raw());
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
