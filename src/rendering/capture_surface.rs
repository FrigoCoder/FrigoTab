//! Top-down DIB used by the fullscreen preview worker.

use std::ffi::c_void;
use std::ptr::null_mut;

use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, HBITMAP, HDC, HGDIOBJ, ReleaseDC, SelectObject,
};

/// Owns the screen DC, memory DC, and selected capture bitmap as one resource.
pub(super) struct CaptureSurface {
    screen_dc: HDC,
    memory_dc: HDC,
    bitmap: HBITMAP,
    previous_bitmap: HGDIOBJ,
    bits: *mut u32,
}

impl CaptureSurface {
    pub(super) fn new(width: i32, height: i32) -> Option<Self> {
        let screen_dc = unsafe { GetDC(null_mut()) };
        if screen_dc.is_null() {
            return None;
        }
        let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
        if memory_dc.is_null() {
            unsafe { ReleaseDC(null_mut(), screen_dc) };
            return None;
        }

        let mut surface = Self {
            screen_dc,
            memory_dc,
            bitmap: null_mut(),
            previous_bitmap: null_mut(),
            bits: null_mut(),
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
        let mut bits: *mut c_void = null_mut();
        surface.bitmap =
            unsafe { CreateDIBSection(screen_dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0) };
        surface.bits = bits.cast();
        if surface.bitmap.is_null() || surface.bits.is_null() {
            return None;
        }

        let previous = unsafe { SelectObject(memory_dc, surface.bitmap as HGDIOBJ) };
        if previous.is_null() || previous == -1isize as HGDIOBJ {
            return None;
        }
        surface.previous_bitmap = previous;
        Some(surface)
    }

    pub(super) fn screen_dc(&self) -> HDC {
        self.screen_dc
    }

    pub(super) fn memory_dc(&self) -> HDC {
        self.memory_dc
    }

    pub(super) fn bits(&self) -> *const u32 {
        self.bits
    }
}

impl Drop for CaptureSurface {
    fn drop(&mut self) {
        unsafe {
            if !self.previous_bitmap.is_null() {
                SelectObject(self.memory_dc, self.previous_bitmap);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap as HGDIOBJ);
            }
            if !self.memory_dc.is_null() {
                DeleteDC(self.memory_dc);
            }
            if !self.screen_dc.is_null() {
                ReleaseDC(null_mut(), self.screen_dc);
            }
        }
    }
}
