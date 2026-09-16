//! Compatible memory DC and selected bitmap for a layered surface.

use std::ffi::c_void;
use std::ptr::NonNull;

use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, HDC, HGDIOBJ, SelectObject,
};

use super::layer_errors::win32_error;

/// Owns a compatible memory DC and its selected bitmap.
pub(super) struct MemorySurface {
    dc: NonNull<c_void>,
    bitmap: Option<NonNull<c_void>>,
    old_bitmap: Option<NonNull<c_void>>,
}

impl MemorySurface {
    pub(super) fn create(screen_dc: HDC, width: i32, height: i32) -> Result<Self, String> {
        let dc = unsafe { CreateCompatibleDC(screen_dc) };
        let dc = NonNull::new(dc).ok_or_else(|| win32_error("CreateCompatibleDC"))?;

        // Construct the guard as soon as the DC exists.  Returning from any
        // later failure path now follows the same cleanup path as a fully
        // initialized surface.
        let mut surface = Self {
            dc,
            bitmap: None,
            old_bitmap: None,
        };

        surface.bitmap = NonNull::new(unsafe { CreateCompatibleBitmap(screen_dc, width, height) });
        let bitmap = surface
            .bitmap
            .ok_or_else(|| win32_error("CreateCompatibleBitmap"))?;

        surface.old_bitmap =
            valid_gdi_object(unsafe { SelectObject(surface.dc(), bitmap.as_ptr()) });
        surface
            .old_bitmap
            .ok_or_else(|| win32_error("SelectObject"))?;

        Ok(surface)
    }

    pub(super) fn dc(&self) -> HDC {
        self.dc.as_ptr()
    }
}

impl Drop for MemorySurface {
    fn drop(&mut self) {
        let mut delete_after_dc = false;
        if let Some(old_bitmap) = self.old_bitmap.take() {
            let restored = unsafe { SelectObject(self.dc(), old_bitmap.as_ptr()) };
            if invalid_gdi_object(restored) {
                delete_after_dc = true;
            } else if let Some(bitmap) = self.bitmap.take() {
                if unsafe { DeleteObject(bitmap.as_ptr()) } == 0 {
                    delete_after_dc = true;
                    self.bitmap = Some(bitmap);
                } else {
                    self.bitmap = None;
                }
            }
        } else if self.bitmap.is_some() {
            delete_after_dc = true;
        }

        unsafe {
            DeleteDC(self.dc());
        }

        if delete_after_dc && let Some(bitmap) = self.bitmap.take() {
            unsafe {
                DeleteObject(bitmap.as_ptr());
            }
        }
    }
}

fn invalid_gdi_object(value: HGDIOBJ) -> bool {
    value.is_null() || value == (-1isize as HGDIOBJ)
}

fn valid_gdi_object(value: HGDIOBJ) -> Option<NonNull<c_void>> {
    (!invalid_gdi_object(value))
        .then(|| NonNull::new(value))
        .flatten()
}
