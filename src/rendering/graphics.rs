//! GDI+ graphics surface backed by a Win32 memory DC.

use std::ptr::{NonNull, null, null_mut};

use windows_sys::Win32::Graphics::Gdi::{HDC, IntersectClipRect, RestoreDC, SaveDC};
use windows_sys::Win32::Graphics::GdiPlus::{
    FillModeAlternate, GdipCreateFromHDC, GdipDeleteGraphics, GdipDrawString, GdipFillPolygon,
    GdipGraphicsClear, GdipMeasureString, GdipSetPixelOffsetMode, GdipSetSmoothingMode,
    GdipSetTextRenderingHint, GpGraphics, PixelOffsetModeHighQuality, PointF, RectF,
    SmoothingModeAntiAlias, TextRenderingHintAntiAliasGridFit,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{DI_NORMAL, DrawIconEx, HICON};

use super::error::{Error, Result, check, check_drawing};
use super::font::Font;
use super::graphics_dc::GraphicsDc;
use super::solid_brush::SolidBrush;
use super::startup::ensure_started;

/// A GDI+ graphics object backed by a Win32 memory DC.
pub struct Graphics {
    pub(super) raw: NonNull<GpGraphics>,
    pub width: f32,
    pub height: f32,
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

    pub(super) fn raw_ptr(&self) -> *mut GpGraphics {
        self.raw.as_ptr()
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
            windows_sys::Win32::Foundation::SetLastError(0);
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
            windows_sys::Win32::Foundation::SetLastError(0);
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
