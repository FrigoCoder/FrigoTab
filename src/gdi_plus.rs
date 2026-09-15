//! The small GDI+ surface used by the application tiles.
//!
//! The application renders its title and number through GDI+. Keeping these
//! calls in one file makes the native renderer use the same path rather than
//! silently falling back to GDI `TextOut`.

use std::ptr::{NonNull, null, null_mut};
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
    raw: NonNull<GpGraphics>,
    pub width: f32,
    pub height: f32,
}

/// A temporary HDC borrowed from a GDI+ graphics object.
///
/// GDI+ requires every successful `GdipGetDC` call to be paired with
/// `GdipReleaseDC` before the graphics object is used again.  Keeping the
/// graphics borrow in this guard prevents the object from being mutably used
/// or dropped while the HDC is outstanding.  The explicit `release` method
/// preserves the original error reporting; `Drop` is the panic/early-return
/// fallback.
struct GraphicsDc<'a> {
    graphics: &'a Graphics,
    dc: NonNull<std::ffi::c_void>,
    released: bool,
}

impl<'a> GraphicsDc<'a> {
    fn acquire(graphics: &'a Graphics) -> Result<Self> {
        let mut dc = null_mut();
        check(unsafe { GdipGetDC(graphics.raw.as_ptr(), &mut dc) })?;
        let dc = NonNull::new(dc).ok_or(Error(1))?;
        Ok(Self {
            graphics,
            dc,
            released: false,
        })
    }

    fn handle(&self) -> HDC {
        self.dc.as_ptr()
    }

    fn release(mut self) -> Result<()> {
        let status = unsafe { GdipReleaseDC(self.graphics.raw.as_ptr(), self.handle()) };
        self.released = true;
        check(status)
    }
}

impl Drop for GraphicsDc<'_> {
    fn drop(&mut self) {
        if !self.released {
            unsafe {
                let _ = GdipReleaseDC(self.graphics.raw.as_ptr(), self.handle());
            }
        }
    }
}

/// Owns a GDI+ solid brush.  Keeping the native handle in a non-null RAII
/// wrapper makes the temporary brush cleanup unconditional, including when a
/// drawing operation is extended with an early return later.
struct SolidBrush(NonNull<GpSolidFill>);

impl SolidBrush {
    fn new(argb: u32) -> Result<Self> {
        let mut raw = null_mut();
        check(unsafe { GdipCreateSolidFill(argb, &mut raw) })?;
        NonNull::new(raw).map(Self).ok_or(Error(1))
    }

    fn as_brush(&self) -> *mut GpBrush {
        self.0.as_ptr().cast()
    }
}

impl Drop for SolidBrush {
    fn drop(&mut self) {
        unsafe {
            let _ = GdipDeleteBrush(self.as_brush());
        }
    }
}

impl Graphics {
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HDC is an opaque GDI handle.
    pub fn from_hdc(hdc: HDC, width: i32, height: i32) -> Result<Self> {
        ensure_started()?;
        let mut raw = null_mut();
        check(unsafe { GdipCreateFromHDC(hdc, &mut raw) })?;
        let raw = NonNull::new(raw).ok_or(Error(1))?;
        Ok(Self {
            raw,
            width: width as f32,
            height: height as f32,
        })
    }

    /// Clear the surface with the transparent color used by the renderer.
    pub fn clear(&mut self) -> Result<()> {
        check(unsafe { GdipGraphicsClear(self.raw.as_ptr(), 0) })
    }

    /// Matches the exact quality flags set by `ApplicationWindow.RenderOverlay`.
    pub fn set_overlay_quality(&mut self) -> Result<()> {
        check(unsafe { GdipSetPixelOffsetMode(self.raw.as_ptr(), PixelOffsetModeHighQuality) })?;
        check(unsafe { GdipSetSmoothingMode(self.raw.as_ptr(), SmoothingModeAntiAlias) })?;
        Ok(())
    }

    pub fn set_text_rendering_hint(&mut self) -> Result<()> {
        check(unsafe {
            GdipSetTextRenderingHint(self.raw.as_ptr(), TextRenderingHintAntiAliasGridFit)
        })
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
        let brush = SolidBrush::new(argb)?;
        check_drawing(unsafe {
            SetLastError(0);
            GdipFillPolygon(
                self.raw.as_ptr(),
                brush.as_brush(),
                points.as_ptr(),
                points.len() as i32,
                FillModeAlternate,
            )
        })
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
                self.raw.as_ptr(),
                utf16.as_ptr(),
                utf16.len() as i32,
                font.raw.as_ptr(),
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
        let brush = SolidBrush::new(argb)?;
        check_drawing(unsafe {
            SetLastError(0);
            GdipDrawString(
                self.raw.as_ptr(),
                utf16.as_ptr(),
                utf16.len() as i32,
                font.raw.as_ptr(),
                &rect,
                null(),
                brush.as_brush(),
            )
        })
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
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HICON is an opaque User32 handle.
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
        let dc = GraphicsDc::acquire(self)?;
        unsafe {
            // The original icon drawing path does not surface DrawIconEx's
            // BOOL result to the caller.
            let saved_dc = SaveDC(dc.handle());
            IntersectClipRect(
                dc.handle(),
                x,
                y,
                x.saturating_add(width),
                y.saturating_add(height),
            );
            DrawIconEx(
                dc.handle(),
                x,
                y,
                icon,
                width,
                height,
                0,
                null_mut(),
                DI_NORMAL,
            );
            if saved_dc != 0 {
                RestoreDC(dc.handle(), saved_dc);
            }
        }
        dc.release()
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        unsafe {
            let _ = GdipDeleteGraphics(self.raw.as_ptr());
        }
    }
}

/// Owns a GDI+ font family. GDI+ fonts borrow their family, so this wrapper is
/// stored directly in `Font` and dropped after the font itself.
struct FontFamily(NonNull<GpFontFamily>);

impl FontFamily {
    fn new(name: &str) -> Result<Self> {
        let utf16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut raw = null_mut();
        check(unsafe { GdipCreateFontFamilyFromName(utf16.as_ptr(), null_mut(), &mut raw) })?;
        NonNull::new(raw).map(Self).ok_or(Error(1))
    }

    fn as_ptr(&self) -> *mut GpFontFamily {
        self.0.as_ptr()
    }
}

impl Drop for FontFamily {
    fn drop(&mut self) {
        unsafe {
            let _ = GdipDeleteFontFamily(self.as_ptr());
        }
    }
}

/// A GDI+ font family and font pair. The family is kept alongside the font
/// because GDI+ fonts borrow it.
pub struct Font {
    raw: NonNull<GpFont>,
    _family: FontFamily,
}

impl Font {
    pub fn new(name: &str, size: f32, style: FontStyle) -> Result<Self> {
        ensure_started()?;
        let family = FontFamily::new(name)?;
        let mut raw = null_mut();
        check(unsafe { GdipCreateFont(family.as_ptr(), size, style, UnitPoint, &mut raw) })?;
        let raw = NonNull::new(raw).ok_or(Error(1))?;
        Ok(Self {
            raw,
            _family: family,
        })
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            let _ = GdipDeleteFont(self.raw.as_ptr());
        }
    }
}
