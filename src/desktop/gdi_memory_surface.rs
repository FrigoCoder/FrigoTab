use std::ffi::c_void;
use std::ptr::{NonNull, null_mut};

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, HDC, HGDIOBJ, SelectObject,
};

use super::diagnostics::write_diagnostic;
use super::geometry::{height, width};

/// Owns a memory DC and the bitmap selected into it.
///
/// GDI requires the selected bitmap to be restored before it is deleted.  The
/// guard also owns partially-created surfaces, so every failure after the DC
/// is created follows the same cleanup path as a successfully published
/// surface.
pub(super) struct GdiMemorySurface {
    dc: GdiHandle,
    bitmap: Option<GdiHandle>,
    previous_bitmap: Option<GdiHandle>,
}

type GdiHandle = NonNull<c_void>;

impl GdiMemorySurface {
    pub(super) fn new_dc(source_dc: HDC, name: &str) -> Result<Self, ()> {
        let dc = unsafe { CreateCompatibleDC(source_dc) };
        let Some(dc) = NonNull::new(dc) else {
            write_diagnostic(&format!("CreateCompatibleDC failed for the {name}"));
            return Err(());
        };
        Ok(Self {
            dc,
            bitmap: None,
            previous_bitmap: None,
        })
    }

    pub(super) fn new_compatible(source_dc: HDC, width: i32, height: i32) -> Result<Self, ()> {
        let mut surface = Self::new_dc(source_dc, "shell print surface")?;
        surface.bitmap = NonNull::new(unsafe { CreateCompatibleBitmap(source_dc, width, height) });
        if surface.bitmap.is_none() {
            write_diagnostic("CreateCompatibleBitmap failed for the shell print surface");
            return Err(());
        }
        surface.select("shell print surface")
    }

    pub(super) fn new_dib(source_dc: HDC, source_bounds: RECT) -> Result<Self, ()> {
        let mut surface = Self::new_dc(source_dc, "shell snapshot")?;
        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width(source_bounds),
                // A negative height creates a top-down DIB whose first
                // scan line is the top of the virtual desktop.
                biHeight: -height(source_bounds),
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = null_mut();
        surface.bitmap = NonNull::new(unsafe {
            CreateDIBSection(
                source_dc,
                &bitmap_info,
                DIB_RGB_COLORS,
                &mut bits,
                null_mut(),
                0,
            )
        });
        if surface.bitmap.is_none() {
            write_diagnostic("CreateDIBSection failed for the shell snapshot");
            return Err(());
        }
        if bits.is_null() {
            write_diagnostic("CreateDIBSection returned no writable shell-snapshot pixels");
            return Err(());
        }
        surface.select("shell snapshot")
    }

    fn select(mut self, name: &str) -> Result<Self, ()> {
        let Some(bitmap) = self.bitmap else {
            write_diagnostic(&format!("SelectObject failed for the {name}"));
            return Err(());
        };
        self.previous_bitmap =
            valid_gdi_handle(unsafe { SelectObject(self.dc(), bitmap.as_ptr()) });
        if self.previous_bitmap.is_none() {
            write_diagnostic(&format!("SelectObject failed for the {name}"));
            return Err(());
        }
        Ok(self)
    }

    pub(super) fn dc(&self) -> HDC {
        self.dc.as_ptr()
    }
}

impl Drop for GdiMemorySurface {
    fn drop(&mut self) {
        let mut bitmap_deselected = true;
        if let Some(previous_bitmap) = self.previous_bitmap {
            let restored = unsafe { SelectObject(self.dc(), previous_bitmap.as_ptr()) };
            bitmap_deselected = !invalid_gdi_object(restored);
            if !bitmap_deselected {
                write_diagnostic("SelectObject failed while releasing the shell snapshot");
            }
        }
        self.previous_bitmap = None;

        if let Some(bitmap) = self.bitmap.take() {
            if bitmap_deselected {
                if unsafe { DeleteObject(bitmap.as_ptr()) } == 0 {
                    write_diagnostic("DeleteObject failed while releasing the shell snapshot");
                }
            } else {
                self.bitmap = Some(bitmap);
            }
        }
        if unsafe { DeleteDC(self.dc()) } == 0 {
            write_diagnostic("DeleteDC failed while releasing the shell snapshot");
        }
        if let Some(bitmap) = self.bitmap.take() {
            // A selected bitmap cannot be deleted until its memory DC is gone.
            // Destroying that DC above releases the selection so this retry
            // is safe and matches the original cleanup path.
            if unsafe { DeleteObject(bitmap.as_ptr()) } == 0 {
                write_diagnostic("DeleteObject failed after releasing the shell snapshot DC");
            }
        }
    }
}

fn invalid_gdi_object(value: HGDIOBJ) -> bool {
    value.is_null() || value == (-1isize as *mut c_void)
}

fn valid_gdi_handle(value: HGDIOBJ) -> Option<GdiHandle> {
    if invalid_gdi_object(value) {
        None
    } else {
        NonNull::new(value)
    }
}
