//! Native DWM thumbnail wrapper used by the application.
//!
//! A thumbnail is registered hidden.  The owner paints its background first,
//! then calls [`DwmThumbnail::set_visible`] for the completed collection.  All
//! three native operations are checked for failed HRESULTs, including the
//! best-effort unregister performed by `Drop`.

use std::num::NonZeroIsize;

use windows_sys::Win32::Foundation::{E_FAIL, HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::{
    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION, DWM_TNP_RECTSOURCE,
    DWM_TNP_VISIBLE, DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
};
use windows_sys::Win32::System::Diagnostics::Debug::OutputDebugStringW;

/// One DWM thumbnail registration.
///
/// This intentionally mirrors the original `Thumbnail` contract: registration starts
/// hidden, source/destination updates set an opaque thumbnail, and visibility
/// is a separate update.  The handle is owned by this value and is released
/// when it is dropped.
pub struct DwmThumbnail {
    handle: NonZeroIsize,
}

impl DwmThumbnail {
    /// Register `source` as a thumbnail hosted by `destination`.
    ///
    /// DWM registrations are hidden by default.  The caller must set the
    /// source and destination rectangles and explicitly make the thumbnail
    /// visible after the owner has painted its backdrop.
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND values are opaque window identities.
    pub fn register(destination: HWND, source: HWND) -> Result<Self, i32> {
        let mut handle = 0isize;
        let hresult = unsafe { DwmRegisterThumbnail(destination, source, &mut handle) };
        if hresult < 0 {
            // The native API normally leaves the output null on failure, but
            // release it if a platform ever returns both a failure and a
            // non-null registration.
            if handle != 0 {
                let release_result = unsafe { DwmUnregisterThumbnail(handle) };
                if release_result < 0 {
                    write_diagnostic(
                        "DwmUnregisterThumbnail after registration failure",
                        release_result,
                    );
                }
            }
            return Err(hresult);
        }
        let Some(handle) = NonZeroIsize::new(handle) else {
            // The original implementation rejects a successful call that returns a
            // null handle.  E_FAIL is used here because there is no HRESULT
            // associated with that malformed result.
            return Err(E_FAIL);
        };
        Ok(Self { handle })
    }

    /// Set the source rectangle copied from the source window.
    pub fn set_source_rect(&self, source: RECT) -> Result<(), i32> {
        let properties = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTSOURCE | DWM_TNP_OPACITY,
            rcSource: source,
            opacity: u8::MAX,
            ..Default::default()
        };
        self.update(properties)
    }

    /// Set the destination rectangle in the owner window.
    pub fn set_destination_rect(&self, destination: RECT) -> Result<(), i32> {
        let properties = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_OPACITY,
            rcDestination: destination,
            opacity: u8::MAX,
            ..Default::default()
        };
        self.update(properties)
    }

    /// Make the registration visible or hidden.
    pub fn set_visible(&self, visible: bool) -> Result<(), i32> {
        let properties = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_VISIBLE,
            fVisible: if visible { 1 } else { 0 },
            ..Default::default()
        };
        self.update(properties)
    }

    /// Expose the native registration for diagnostics only.
    pub fn handle(&self) -> isize {
        self.handle.get()
    }

    fn update(&self, properties: DWM_THUMBNAIL_PROPERTIES) -> Result<(), i32> {
        let hresult = unsafe { DwmUpdateThumbnailProperties(self.handle.get(), &properties) };
        if hresult < 0 {
            write_diagnostic("DwmUpdateThumbnailProperties", hresult);
            Err(hresult)
        } else {
            Ok(())
        }
    }
}

impl Drop for DwmThumbnail {
    fn drop(&mut self) {
        let hresult = unsafe { DwmUnregisterThumbnail(self.handle.get()) };
        if hresult < 0 {
            write_diagnostic("DwmUnregisterThumbnail", hresult);
        }
    }
}

fn write_diagnostic(operation: &str, hresult: i32) {
    let message = format!("{operation} failed with HRESULT 0x{:08X}\0", hresult as u32);
    let wide: Vec<u16> = message.encode_utf16().collect();
    unsafe { OutputDebugStringW(wide.as_ptr()) };
}
