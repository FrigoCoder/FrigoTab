use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, GetPixel, HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject,
};

use crate::{Color, color_distance};

pub struct ScreenCapture {
    bounds: RECT,
    pixels: Vec<Color>,
}

impl ScreenCapture {
    pub fn bounds(&self) -> RECT {
        self.bounds
    }
    pub fn width(&self) -> i32 {
        self.bounds.right - self.bounds.left
    }
    pub fn height(&self) -> i32 {
        self.bounds.bottom - self.bounds.top
    }
    pub fn pixel(&self, x: i32, y: i32) -> Option<Color> {
        if x < self.bounds.left
            || x >= self.bounds.right
            || y < self.bounds.top
            || y >= self.bounds.bottom
        {
            return None;
        }
        let index = (y - self.bounds.top) as usize * self.width() as usize
            + (x - self.bounds.left) as usize;
        self.pixels.get(index).copied()
    }
    pub fn find(&self, expected: Color, tolerance: u32) -> Option<POINT> {
        for y in self.bounds.top..self.bounds.bottom {
            for x in self.bounds.left..self.bounds.right {
                if self
                    .pixel(x, y)
                    .is_some_and(|color| color_distance(color, expected) <= tolerance)
                {
                    return Some(POINT { x, y });
                }
            }
        }
        None
    }
}

pub fn capture_screen_image(bounds: RECT) -> Option<ScreenCapture> {
    let width = bounds.right.saturating_sub(bounds.left);
    let height = bounds.bottom.saturating_sub(bounds.top);
    if width <= 0 || height <= 0 {
        return None;
    }
    let screen = unsafe { GetDC(null_mut()) };
    if screen.is_null() {
        return None;
    }
    let memory = unsafe { CreateCompatibleDC(screen) };
    if memory.is_null() {
        unsafe { ReleaseDC(null_mut(), screen) };
        return None;
    }
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            ..unsafe { zeroed() }
        },
        ..unsafe { zeroed() }
    };
    let mut bits = null_mut();
    let bitmap =
        unsafe { CreateDIBSection(screen, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0) };
    if bitmap.is_null() || bits.is_null() {
        if !bitmap.is_null() {
            unsafe { DeleteObject(bitmap) };
        }
        unsafe {
            DeleteDC(memory);
            ReleaseDC(null_mut(), screen);
        }
        return None;
    }
    let previous = unsafe { SelectObject(memory, bitmap as HGDIOBJ) };
    let copied = unsafe {
        BitBlt(
            memory,
            0,
            0,
            width,
            height,
            screen,
            bounds.left,
            bounds.top,
            SRCCOPY,
        )
    };
    let pixels = if copied != 0 {
        unsafe { std::slice::from_raw_parts(bits as *const Color, (width * height) as usize) }
            .iter()
            .map(|pixel| {
                let red = (pixel >> 16) & 0xff;
                let green = (pixel >> 8) & 0xff;
                let blue = pixel & 0xff;
                red | (green << 8) | (blue << 16)
            })
            .collect()
    } else {
        Vec::new()
    };
    unsafe {
        if !previous.is_null() {
            SelectObject(memory, previous);
        }
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(null_mut(), screen);
    }
    (copied != 0).then_some(ScreenCapture { bounds, pixels })
}

pub fn capture_screen(bounds: RECT) -> Option<Vec<Color>> {
    capture_screen_image(bounds).map(|capture| capture.pixels)
}

pub fn pixel_at(x: i32, y: i32) -> Option<Color> {
    unsafe {
        let dc = GetDC(null_mut());
        if dc.is_null() {
            return None;
        }
        let color = GetPixel(dc, x, y);
        ReleaseDC(null_mut(), dc);
        (color != u32::MAX).then_some(color)
    }
}

pub fn screen_pixel(x: i32, y: i32) -> Option<Color> {
    pixel_at(x, y)
}
pub fn find_screen_pixel(bounds: RECT, expected: Color, tolerance: u32) -> Option<POINT> {
    capture_screen_image(bounds)?.find(expected, tolerance)
}
pub fn find_pixel(bounds: RECT, expected: Color, tolerance: u32) -> Option<POINT> {
    find_screen_pixel(bounds, expected, tolerance)
}
