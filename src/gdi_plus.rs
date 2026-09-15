//! The small GDI+ surface used by the application tiles.
//!
//! The application renders its title and number through GDI+. Keeping these
//! calls in one file makes the native renderer use the same path rather than
//! silently falling back to GDI `TextOut`.

use std::ptr::{null, null_mut};
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{GetLastError, SetLastError};
use windows_sys::Win32::Graphics::Gdi::{HDC, IntersectClipRect, RestoreDC, SaveDC};
use windows_sys::Win32::Graphics::GdiPlus::{
    FillModeAlternate, FontStyle, GdipCreateFont, GdipCreateFontFamilyFromName, GdipCreateFromHDC,
    GdipCreateSolidFill, GdipDeleteBrush, GdipDeleteFont, GdipDeleteFontFamily, GdipDeleteGraphics,
    GdipDrawString, GdipFillPolygon, GdipGetDC, GdipGraphicsClear, GdipMeasureString,
    GdipReleaseDC, GdipSetPixelOffsetMode, GdipSetSmoothingMode, GdipSetTextRenderingHint,
    GdiplusStartup, GdiplusStartupInput, GpBrush, GpFont, GpFontFamily, GpGraphics, GpSolidFill,
    PixelOffsetModeHighQuality, PointF, RectF, SmoothingModeAntiAlias, Status,
    TextRenderingHintAntiAliasGridFit, UnitPoint,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DI_NORMAL, DrawIconEx, GetSystemMetrics, HICON, SM_REMOTESESSION,
};

/// A GDI+ status code.  GDI+ does not use `GetLastError`; every operation
/// returns one of these values instead.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Error(pub Status);

pub type Result<T> = std::result::Result<T, Error>;

static STARTUP_STATUS: OnceLock<Status> = OnceLock::new();

/// Start GDI+ once for the process.  It is intentionally never shut down:
/// all tile resources are released before process exit, and keeping the token
/// alive for the process lifetime matches the way GDI+ is used by
/// the original application.
pub fn ensure_started() -> Result<()> {
    let status = *STARTUP_STATUS.get_or_init(|| unsafe {
        let mut token = 0usize;
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            DebugEventCallback: 0,
            SuppressBackgroundThread: 0,
            SuppressExternalCodecs: 0,
        };
        GdiplusStartup(&mut token, &input, null_mut())
    });
    if status == 0 {
        Ok(())
    } else {
        Err(Error(status))
    }
}

fn check(status: Status) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(Error(status))
    }
}

// GDI+'s drawing operations deliberately tolerate the GDI+ generic
// and Win32 errors produced when the secure desktop or a remote session makes
// a display surface temporarily unavailable. This is its CheckErrorStatus
// behavior; setup/measurement operations continue to use strict `check`.
fn check_drawing(status: Status) -> Result<()> {
    if status == 0 {
        return Ok(());
    }
    if status == 1 || status == 7 {
        let error = unsafe { GetLastError() };
        if error == 5
            || error == 127
            || (error == 0 && unsafe { GetSystemMetrics(SM_REMOTESESSION) } & 1 != 0)
        {
            return Ok(());
        }
    }
    Err(Error(status))
}

/// A GDI+ graphics object backed by a Win32 memory DC.
pub struct Graphics {
    raw: *mut GpGraphics,
    pub width: f32,
    pub height: f32,
}

impl Graphics {
    pub fn from_hdc(hdc: HDC, width: i32, height: i32) -> Result<Self> {
        ensure_started()?;
        let mut raw = null_mut();
        check(unsafe { GdipCreateFromHDC(hdc, &mut raw) })?;
        if raw.is_null() {
            return Err(Error(1));
        }
        Ok(Self {
            raw,
            width: width as f32,
            height: height as f32,
        })
    }

    /// Clear the surface with the transparent color used by the renderer.
    pub fn clear(&mut self) -> Result<()> {
        check(unsafe { GdipGraphicsClear(self.raw, 0) })
    }

    /// Matches the exact quality flags set by `ApplicationWindow.RenderOverlay`.
    pub fn set_overlay_quality(&mut self) -> Result<()> {
        check(unsafe { GdipSetPixelOffsetMode(self.raw, PixelOffsetModeHighQuality) })?;
        check(unsafe { GdipSetSmoothingMode(self.raw, SmoothingModeAntiAlias) })?;
        Ok(())
    }

    pub fn set_text_rendering_hint(&mut self) -> Result<()> {
        check(unsafe { GdipSetTextRenderingHint(self.raw, TextRenderingHintAntiAliasGridFit) })
    }

    pub fn fill_rect(&mut self, rect: RectF, argb: u32) -> Result<()> {
        // The helper is intentionally implemented with FillPolygon rather
        // than FillRectangle. Preserve its duplicated first point and winding
        // exactly, including the default Alternate fill mode.
        let points = [
            PointF {
                X: rect.X,
                Y: rect.Y,
            },
            PointF {
                X: rect.X,
                Y: rect.Y,
            },
            PointF {
                X: rect.X + rect.Width,
                Y: rect.Y,
            },
            PointF {
                X: rect.X + rect.Width,
                Y: rect.Y + rect.Height,
            },
            PointF {
                X: rect.X,
                Y: rect.Y + rect.Height,
            },
        ];
        self.fill_polygon(&points, argb)
    }

    pub fn fill_polygon(&mut self, points: &[PointF], argb: u32) -> Result<()> {
        if points.is_empty() {
            return Ok(());
        }
        let mut brush: *mut GpSolidFill = null_mut();
        check(unsafe { GdipCreateSolidFill(argb, &mut brush) })?;
        if brush.is_null() {
            return Err(Error(1));
        }
        let result = check_drawing(unsafe {
            SetLastError(0);
            GdipFillPolygon(
                self.raw,
                brush.cast::<GpBrush>(),
                points.as_ptr(),
                points.len() as i32,
                FillModeAlternate,
            )
        });
        unsafe {
            let _ = GdipDeleteBrush(brush.cast::<GpBrush>());
        }
        result
    }

    pub fn measure_string(&mut self, text: &str, font: &Font) -> Result<RectF> {
        if text.is_empty() {
            return Ok(RectF::default());
        }
        let utf16: Vec<u16> = text.encode_utf16().collect();
        let layout = RectF {
            X: 0.0,
            Y: 0.0,
            Width: 0.0,
            Height: 0.0,
        };
        let mut bounds = RectF::default();
        let mut codepoints = 0;
        let mut lines = 0;
        check(unsafe {
            GdipMeasureString(
                self.raw,
                utf16.as_ptr(),
                utf16.len() as i32,
                font.raw,
                &layout,
                null(),
                &mut bounds,
                &mut codepoints,
                &mut lines,
            )
        })?;
        Ok(bounds)
    }

    pub fn draw_string(&mut self, text: &str, font: &Font, rect: RectF, argb: u32) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let utf16: Vec<u16> = text.encode_utf16().collect();
        let mut brush: *mut GpSolidFill = null_mut();
        check(unsafe { GdipCreateSolidFill(argb, &mut brush) })?;
        if brush.is_null() {
            return Err(Error(1));
        }
        let result = check_drawing(unsafe {
            SetLastError(0);
            GdipDrawString(
                self.raw,
                utf16.as_ptr(),
                utf16.len() as i32,
                font.raw,
                &rect,
                null(),
                brush.cast::<GpBrush>(),
            )
        });
        unsafe {
            let _ = GdipDeleteBrush(brush.cast::<GpBrush>());
        }
        result
    }

    /// Matches the zero-sized layout rectangle used by the point overload.
    pub fn draw_string_at(
        &mut self,
        text: &str,
        font: &Font,
        x: f32,
        y: f32,
        argb: u32,
    ) -> Result<()> {
        self.draw_string(
            text,
            font,
            RectF {
                X: x,
                Y: y,
                Width: 0.0,
                Height: 0.0,
            },
            argb,
        )
    }

    /// Draw an icon at integer coordinates through the graphics HDC and
    /// DrawIconEx at the icon's native dimensions; do not convert it to a
    /// GDI+ bitmap.
    pub fn draw_icon(
        &mut self,
        icon: HICON,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        if icon.is_null() {
            return Ok(());
        }
        let mut dc = null_mut();
        check(unsafe { GdipGetDC(self.raw, &mut dc) })?;
        if dc.is_null() {
            return Err(Error(1));
        }
        unsafe {
            // The original icon drawing path does not surface DrawIconEx's
            // BOOL result to the caller.
            let saved_dc = SaveDC(dc);
            IntersectClipRect(dc, x, y, x.saturating_add(width), y.saturating_add(height));
            DrawIconEx(dc, x, y, icon, width, height, 0, null_mut(), DI_NORMAL);
            if saved_dc != 0 {
                RestoreDC(dc, saved_dc);
            }
        }
        check(unsafe { GdipReleaseDC(self.raw, dc) })
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                let _ = GdipDeleteGraphics(self.raw);
            }
            self.raw = null_mut();
        }
    }
}

/// A GDI+ font family and font pair.  The family is kept alongside the font
/// because GDI+ fonts borrow it.
pub struct Font {
    family: *mut GpFontFamily,
    raw: *mut GpFont,
}

impl Font {
    pub fn new(name: &str, size: f32, style: FontStyle) -> Result<Self> {
        ensure_started()?;
        let utf16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut family = null_mut();
        check(unsafe { GdipCreateFontFamilyFromName(utf16.as_ptr(), null_mut(), &mut family) })?;
        if family.is_null() {
            return Err(Error(1));
        }
        let mut raw = null_mut();
        if let Err(error) =
            check(unsafe { GdipCreateFont(family, size, style, UnitPoint, &mut raw) })
        {
            unsafe {
                let _ = GdipDeleteFontFamily(family);
            }
            return Err(error);
        }
        if raw.is_null() {
            unsafe {
                let _ = GdipDeleteFontFamily(family);
            }
            return Err(Error(1));
        }
        Ok(Self { family, raw })
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                let _ = GdipDeleteFont(self.raw);
            }
            if !self.family.is_null() {
                let _ = GdipDeleteFontFamily(self.family);
            }
        }
        self.raw = null_mut();
        self.family = null_mut();
    }
}
