//! Cached frame for a fullscreen window that may stop exposing a DWM surface.

use std::sync::Arc;

use windows_sys::Win32::Graphics::GdiPlus::RectF;

use super::graphics::Graphics;

const ARGB_BLACK: u32 = 0xff00_0000;

/// A fullscreen candidate marker. `image` remains `None` when GDI capture
/// failed, allowing the caller to draw a stable black placeholder.
#[derive(Clone)]
pub struct FullscreenPreview {
    pub(crate) image: Option<Arc<[u32]>>,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) hwnd: isize,
    pub(crate) pid: u32,
}

impl FullscreenPreview {
    pub(crate) fn has_image(&self) -> bool {
        self.image.is_some()
    }

    pub(crate) fn has_same_image(&self, other: &Self) -> bool {
        match (&self.image, &other.image) {
            (Some(left), Some(right)) => Arc::ptr_eq(left, right),
            (None, None) => true,
            _ => false,
        }
    }

    /// Paint an opaque placeholder first, then cover it with the latest frame
    /// when one is available.
    pub(crate) fn draw(&self, graphics: &mut Graphics) -> super::Result<()> {
        graphics.fill_rect(
            RectF {
                X: 0.0,
                Y: 0.0,
                Width: graphics.width,
                Height: graphics.height,
            },
            ARGB_BLACK,
        )?;
        if let Some(pixels) = self.image.as_ref() {
            graphics.draw_argb32_image(pixels, self.width, self.height)?;
        }
        Ok(())
    }
}
