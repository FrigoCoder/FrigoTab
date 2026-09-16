//! Per-pixel alpha surface used by one application tile.

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HWND, POINT, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::BLENDFUNCTION;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowRect, ULW_ALPHA, UpdateLayeredWindow};

use super::gdi_plus::{self, Graphics};
use super::layer_errors::{gdiplus_error, win32_error};
use super::memory_surface::MemorySurface;
use super::screen_dc::ScreenDc;
use crate::window::WindowHandle;

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
