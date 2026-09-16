//! The per-user mutex used by the tray application.
//!
//! This is the native equivalent of `SingleInstanceGuard`: construction does
//! not claim the mutex, a zero-time wait claims it (including an abandoned
//! mutex), and the claim is held until the guard is dropped.

use std::ptr::null;

use windows_sys::Win32::Foundation::{
    ERROR_INVALID_PARAMETER, GetLastError, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

use super::owned_handle::OwnedHandle;

pub const APPLICATION_MUTEX_NAME: &str = "Local\\FrigoTab";

pub use super::single_instance_error::SingleInstanceError;

/// Owns one successful wait on the application mutex.
pub struct SingleInstanceGuard {
    mutex: OwnedHandle,
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
