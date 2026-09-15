//! Per-pixel alpha surface used by one application tile.

use std::ffi::c_void;
use std::ptr::{NonNull, null_mut};

use windows_sys::Win32::Foundation::{GetLastError, HWND, POINT, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    BLENDFUNCTION, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, HDC,
    HGDIOBJ, ReleaseDC, SelectObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowRect, ULW_ALPHA, UpdateLayeredWindow};

use crate::gdi_plus::{self, Graphics};
use crate::window_handle::WindowHandle;

fn win32_error(operation: &str) -> String {
    format!("{operation} failed (Win32 error {})", unsafe {
        GetLastError()
    })
}

fn gdiplus_error(operation: &str, error: gdi_plus::Error) -> String {
    format!("{operation} failed (GDI+ status {})", error.0)
}

/// A screen DC kept alive for the lifetime of its layer surface.
///
/// `GetDC(null_mut())` obtains a DC for the entire screen. The updater keeps
/// that DC instead of reacquiring it for every redraw, matching the original
/// implementation's resource lifetime.
struct ScreenDc {
    handle: NonNull<c_void>,
}

impl ScreenDc {
    fn acquire() -> Result<Self, String> {
        let handle = unsafe { GetDC(null_mut()) };
        NonNull::new(handle)
            .map(|handle| Self { handle })
            .ok_or_else(|| win32_error("GetDC"))
    }

    fn handle(&self) -> HDC {
        self.handle.as_ptr()
    }
}

impl Drop for ScreenDc {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(null_mut(), self.handle());
        }
    }
}

/// Owns a compatible memory DC and its selected bitmap.
///
/// Restoring the DC's previous bitmap before deleting either the DC or the
/// bitmap is important: deleting a selected GDI object is invalid and can
/// leave the process holding the resource indefinitely.
struct MemorySurface {
    dc: NonNull<c_void>,
    bitmap: Option<NonNull<c_void>>,
    old_bitmap: Option<NonNull<c_void>>,
}

impl MemorySurface {
    fn create(screen_dc: HDC, width: i32, height: i32) -> Result<Self, String> {
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

    fn dc(&self) -> HDC {
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

/// Owns the same native resources as the original `LayerUpdater`: a screen
/// DC, memory DC, selected bitmap, and the previous bitmap for restoration.
pub struct LayerUpdater {
    hwnd: HWND,
    surface: MemorySurface,
    // Kept as a field so the screen DC remains valid for the updater's full
    // lifetime, just as it did in the original implementation. It follows
    // `surface` so the dependent memory resources are released first.
    _screen_dc: ScreenDc,
    width: i32,
    height: i32,
}

impl LayerUpdater {
    pub fn new(hwnd: HWND, bounds: RECT) -> Result<Self, String> {
        let width = bounds.right - bounds.left;
        let height = bounds.bottom - bounds.top;
        if width <= 0 || height <= 0 {
            return Err("LayerUpdater requires positive bounds".to_string());
        }

        let screen_dc = ScreenDc::acquire()?;

        // This is intentionally CreateCompatibleBitmap, matching the original
        // LayerUpdater exactly. GDI+ clears and paints the selected surface
        // before UpdateLayeredWindow consumes it.
        let surface = MemorySurface::create(screen_dc.handle(), width, height)?;

        Ok(Self {
            hwnd,
            surface,
            _screen_dc: screen_dc,
            width,
            height,
        })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Paint the transparent DIB with GDI+ and publish it through
    /// UpdateLayeredWindow with source alpha, matching the original updater.
    pub fn update<F>(&mut self, render: F) -> Result<(), String>
    where
        F: FnOnce(&mut Graphics) -> gdi_plus::Result<()>,
    {
        // LayerUpdater.Update returns immediately once its Form is disposed.
        // This also makes a late WM_GETICON callback harmless after teardown.
        if !WindowHandle::new(self.hwnd).is_valid() {
            return Ok(());
        }
        let mut graphics = Graphics::from_hdc(self.surface.dc(), self.width, self.height)
            .map_err(|error| gdiplus_error("GdipCreateFromHDC", error))?;
        graphics
            .clear()
            .map_err(|error| gdiplus_error("GdipGraphicsClear", error))?;
        render(&mut graphics).map_err(|error| gdiplus_error("tile render", error))?;
        drop(graphics);
        self.update_layered_window()
    }

    fn update_layered_window(&self) -> Result<(), String> {
        // The original implementation reads the current window bounds for
        // every publication rather than retaining the constructor rectangle.
        let mut bounds = RECT::default();
        if unsafe { GetWindowRect(self.hwnd, &mut bounds) } == 0 {
            return Err(win32_error("GetWindowRect"));
        }
        let destination = POINT {
            x: bounds.left,
            y: bounds.top,
        };
        let size = SIZE {
            cx: bounds.right - bounds.left,
            cy: bounds.bottom - bounds.top,
        };
        let source = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: 0,
            BlendFlags: 0,
            SourceConstantAlpha: 0xff,
            AlphaFormat: 1,
        };
        if unsafe {
            UpdateLayeredWindow(
                self.hwnd,
                null_mut(),
                &destination,
                &size,
                self.surface.dc(),
                &source,
                0,
                &blend,
                ULW_ALPHA,
            )
        } == 0
        {
            return Err(win32_error("UpdateLayeredWindow"));
        }
        Ok(())
    }
}
