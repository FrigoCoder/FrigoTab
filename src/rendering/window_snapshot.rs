//! A one-frame image of the currently visible foreground window.
//!
//! This is intended for fullscreen sources whose render surface collapses
//! when the switcher appears. The caller must capture it before showing any
//! FrigoTab window; the result is static and can safely be shared by tiles.

use std::sync::Arc;

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, HALFTONE, HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject,
    SetBrushOrgEx, SetStretchBltMode, StretchBlt,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetSystemMetrics, GetWindowRect, IsIconic, IsWindow, IsWindowVisible,
    SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use super::graphics::Graphics;

/// A scaled, opaque snapshot of a foreground window.
#[derive(Clone)]
pub struct WindowSnapshot {
    pixels: Arc<[u32]>,
    width: i32,
    height: i32,
}

impl WindowSnapshot {
    /// Capture `source` from the visible screen into the requested tile size.
    ///
    /// The source must still be the foreground window both before and after
    /// the copy. This prevents accidentally using pixels from a replacement
    /// foreground window during an Alt-Tab race.
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque window identity.
    pub fn capture_foreground(source: HWND, destination: RECT) -> Option<Self> {
        let width = destination.right.checked_sub(destination.left)?;
        let height = destination.bottom.checked_sub(destination.top)?;
        if source.is_null()
            || width <= 0
            || height <= 0
            || width > 16_384
            || height > 16_384
            || unsafe { GetForegroundWindow() } != source
            || unsafe { IsWindow(source) } == 0
            || unsafe { IsWindowVisible(source) } == 0
            || unsafe { IsIconic(source) } != 0
        {
            return None;
        }

        let mut source_bounds = RECT::default();
        if unsafe { GetWindowRect(source, &mut source_bounds) } == 0 {
            return None;
        }
        let source_width = source_bounds.right.checked_sub(source_bounds.left)?;
        let source_height = source_bounds.bottom.checked_sub(source_bounds.top)?;
        if source_width <= 0 || source_height <= 0 {
            return None;
        }
        let virtual_bounds = virtual_screen_bounds();
        if source_bounds.left < virtual_bounds.left
            || source_bounds.top < virtual_bounds.top
            || source_bounds.right > virtual_bounds.right
            || source_bounds.bottom > virtual_bounds.bottom
        {
            return None;
        }

        let pixel_count = (width as usize).checked_mul(height as usize)?;
        let surface = CaptureSurface::new(width, height)?;
        unsafe {
            if SetStretchBltMode(surface.memory_dc(), HALFTONE) == 0 {
                return None;
            }
            let mut brush_origin = POINT::default();
            if SetBrushOrgEx(surface.memory_dc(), 0, 0, &mut brush_origin) == 0
                || StretchBlt(
                    surface.memory_dc(),
                    0,
                    0,
                    width,
                    height,
                    surface.screen_dc(),
                    source_bounds.left,
                    source_bounds.top,
                    source_width,
                    source_height,
                    SRCCOPY,
                ) == 0
            {
                return None;
            }
        }

        // The DIB is top-down and 32-bit BI_RGB. On little-endian Windows its
        // bytes are B,G,R,unused; set that last byte to opaque alpha for GDI+.
        let pixels = unsafe { std::slice::from_raw_parts(surface.bits(), pixel_count) };
        if pixels.iter().all(|pixel| pixel & 0x00ff_ffff == 0) {
            return None;
        }
        let pixels: Arc<[u32]> = pixels
            .iter()
            .map(|pixel| pixel | 0xff00_0000)
            .collect::<Vec<_>>()
            .into();

        let mut current_bounds = RECT::default();
        if unsafe { GetForegroundWindow() } != source
            || unsafe { IsWindow(source) } == 0
            || unsafe { GetWindowRect(source, &mut current_bounds) } == 0
            || !same_rect(source_bounds, current_bounds)
        {
            return None;
        }

        Some(Self {
            pixels,
            width,
            height,
        })
    }

    /// Draw the image across the full tile surface.
    pub fn draw(&self, graphics: &mut Graphics) -> super::error::Result<()> {
        graphics.draw_argb32_image(&self.pixels, self.width, self.height)
    }
}

struct CaptureSurface {
    screen: *mut std::ffi::c_void,
    memory: *mut std::ffi::c_void,
    bitmap: *mut std::ffi::c_void,
    previous_bitmap: *mut std::ffi::c_void,
    bits: *mut u32,
}

impl CaptureSurface {
    fn new(width: i32, height: i32) -> Option<Self> {
        let screen = unsafe { GetDC(std::ptr::null_mut()) };
        if screen.is_null() {
            return None;
        }
        let memory = unsafe { CreateCompatibleDC(screen) };
        if memory.is_null() {
            unsafe { ReleaseDC(std::ptr::null_mut(), screen) };
            return None;
        }
        let mut surface = Self {
            screen,
            memory,
            bitmap: std::ptr::null_mut(),
            previous_bitmap: std::ptr::null_mut(),
            bits: std::ptr::null_mut(),
        };
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        surface.bitmap = unsafe {
            CreateDIBSection(
                screen,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            )
        };
        surface.bits = bits.cast();
        if surface.bitmap.is_null() || surface.bits.is_null() {
            return None;
        }
        surface.previous_bitmap =
            unsafe { SelectObject(surface.memory, surface.bitmap as HGDIOBJ) };
        if surface.previous_bitmap.is_null() || surface.previous_bitmap == -1isize as HGDIOBJ {
            // The bitmap was not selected, so Drop must not try to restore an
            // invalid sentinel as though it were the previous GDI object.
            surface.previous_bitmap = std::ptr::null_mut();
            return None;
        }
        Some(surface)
    }

    fn screen_dc(&self) -> windows_sys::Win32::Graphics::Gdi::HDC {
        self.screen
    }

    fn memory_dc(&self) -> windows_sys::Win32::Graphics::Gdi::HDC {
        self.memory
    }

    fn bits(&self) -> *const u32 {
        self.bits
    }
}

impl Drop for CaptureSurface {
    fn drop(&mut self) {
        unsafe {
            if !self.previous_bitmap.is_null() {
                SelectObject(self.memory, self.previous_bitmap as HGDIOBJ);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap as HGDIOBJ);
            }
            if !self.memory.is_null() {
                DeleteDC(self.memory);
            }
            if !self.screen.is_null() {
                ReleaseDC(std::ptr::null_mut(), self.screen);
            }
        }
    }
}

fn virtual_screen_bounds() -> RECT {
    let (left, top, width, height) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    RECT {
        left,
        top,
        right: left.saturating_add(width),
        bottom: top.saturating_add(height),
    }
}

fn same_rect(left: RECT, right: RECT) -> bool {
    left.left == right.left
        && left.top == right.top
        && left.right == right.right
        && left.bottom == right.bottom
}
