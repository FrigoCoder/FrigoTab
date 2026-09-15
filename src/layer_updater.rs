//! Per-pixel alpha surface used by one application tile.

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{GetLastError, HWND, POINT, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    BLENDFUNCTION, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
    HBITMAP, HDC, HGDIOBJ, ReleaseDC, SelectObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, IsWindow, ULW_ALPHA, UpdateLayeredWindow,
};

use crate::gdi_plus::{self, Graphics};

fn win32_error(operation: &str) -> String {
    format!("{operation} failed (Win32 error {})", unsafe {
        GetLastError()
    })
}

fn gdiplus_error(operation: &str, error: gdi_plus::Error) -> String {
    format!("{operation} failed (GDI+ status {})", error.0)
}

/// Owns the same native resources as the original `LayerUpdater`: a screen DC,
/// memory DC, selected bitmap, and the previous bitmap for restoration.
pub struct LayerUpdater {
    hwnd: HWND,
    bounds: RECT,
    screen_dc: HDC,
    mem_dc: HDC,
    bitmap: HBITMAP,
    old_bitmap: HGDIOBJ,
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

        let screen_dc = unsafe { GetDC(null_mut()) };
        if screen_dc.is_null() {
            return Err(win32_error("GetDC"));
        }
        let mem_dc = unsafe { CreateCompatibleDC(screen_dc) };
        if mem_dc.is_null() {
            unsafe {
                ReleaseDC(null_mut(), screen_dc);
            }
            return Err(win32_error("CreateCompatibleDC"));
        }

        // This is intentionally CreateCompatibleBitmap, matching the original
        // LayerUpdater exactly. GDI+ clears and paints the selected surface
        // before UpdateLayeredWindow consumes it.
        let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width, height) };
        if bitmap.is_null() {
            unsafe {
                DeleteDC(mem_dc);
                ReleaseDC(null_mut(), screen_dc);
            }
            return Err(win32_error("CreateCompatibleBitmap"));
        }

        let old_bitmap = unsafe { SelectObject(mem_dc, bitmap.cast()) };
        if old_bitmap.is_null() || old_bitmap == (-1isize as HGDIOBJ) {
            unsafe {
                DeleteObject(bitmap.cast());
                DeleteDC(mem_dc);
                ReleaseDC(null_mut(), screen_dc);
            }
            return Err(win32_error("SelectObject"));
        }

        Ok(Self {
            hwnd,
            bounds,
            screen_dc,
            mem_dc,
            bitmap,
            old_bitmap,
            width,
            height,
        })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn bounds(&self) -> RECT {
        self.bounds
    }

    /// Paint the transparent DIB with GDI+ and publish it through
    /// UpdateLayeredWindow with source alpha, matching the original updater.
    pub fn update<F>(&mut self, render: F) -> Result<(), String>
    where
        F: FnOnce(&mut Graphics) -> gdi_plus::Result<()>,
    {
        // LayerUpdater.Update returns immediately once its Form is disposed.
        // This also makes a late WM_GETICON callback harmless after teardown.
        if self.hwnd.is_null() || unsafe { IsWindow(self.hwnd) } == 0 {
            return Ok(());
        }
        let mut graphics = Graphics::from_hdc(self.mem_dc, self.width, self.height)
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
                self.mem_dc,
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

impl Drop for LayerUpdater {
    fn drop(&mut self) {
        unsafe {
            if !self.mem_dc.is_null()
                && !self.old_bitmap.is_null()
                && self.old_bitmap != (-1isize as HGDIOBJ)
            {
                SelectObject(self.mem_dc, self.old_bitmap);
            }
            if !self.mem_dc.is_null() {
                DeleteDC(self.mem_dc);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap.cast());
            }
            if !self.screen_dc.is_null() {
                ReleaseDC(null_mut(), self.screen_dc);
            }
        }
        self.bitmap = null_mut();
        self.mem_dc = null_mut();
        self.screen_dc = null_mut();
    }
}
