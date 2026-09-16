//! Native Explorer desktop snapshot used by the application.
//!
//! The source is Explorer's desktop view, never the screen DC.  This matters
//! because a screen capture would copy whichever application windows happen to
//! cover the wallpaper.  Explorer is rendered once into a retained top-down
//! DIB and that DIB is reused while the switcher is shown.

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{BLACKNESS, HDC, PatBlt};

use super::diagnostics::write_diagnostic;
use super::gdi_shell_desktop_snapshot_frame::GdiShellDesktopSnapshotFrame;
use super::geometry::{height, width};

/// A retained Explorer desktop image.
///
/// Capture failures intentionally produce an unavailable snapshot.  `draw`
/// then paints the same opaque black fallback used by the original session
/// window rather than
/// exposing stale pixels or falling back to a screen capture.
pub struct ShellDesktopSnapshot {
    frame: Option<GdiShellDesktopSnapshotFrame>,
}

// The capture worker relinquishes all access before sending these GDI handles
// to the UI thread. Their use is serialized exactly as in the original worker
// BeginInvoke publication path.
unsafe impl Send for ShellDesktopSnapshot {}

impl ShellDesktopSnapshot {
    /// Capture the Explorer desktop for the requested virtual-desktop bounds.
    ///
    /// The constructor is fail-open: Explorer may be restarting, unavailable
    /// on a secure/RDP desktop, or temporarily report stale geometry.  In all
    /// of those cases the returned snapshot is unavailable and `draw` paints
    /// black.
    pub fn capture(source_bounds: RECT) -> Self {
        let frame = if width(source_bounds) > 0 && height(source_bounds) > 0 {
            GdiShellDesktopSnapshotFrame::new(source_bounds).ok()
        } else {
            None
        };
        Self { frame }
    }

    pub fn is_available(&self) -> bool {
        self.frame.is_some()
    }

    /// Paint the retained shell image into `destination_dc`.
    ///
    /// This method deliberately has no screen-capture fallback.  A lost shell
    /// surface, invalid destination, or GDI failure is rendered as opaque
    /// black so application windows can never leak into the background.
    pub fn draw(&self, destination_dc: HDC, destination_bounds: RECT) {
        if destination_dc.is_null()
            || width(destination_bounds) <= 0
            || height(destination_bounds) <= 0
        {
            return;
        }

        let Some(frame) = &self.frame else {
            paint_black(destination_dc, destination_bounds);
            return;
        };

        if frame.draw(destination_dc, destination_bounds).is_err() {
            write_diagnostic("Painting the shell desktop snapshot failed");
            paint_black(destination_dc, destination_bounds);
        }
    }
}

fn paint_black(destination_dc: HDC, destination_bounds: RECT) {
    if !destination_dc.is_null() && width(destination_bounds) > 0 && height(destination_bounds) > 0
    {
        let _ = unsafe {
            PatBlt(
                destination_dc,
                destination_bounds.left,
                destination_bounds.top,
                width(destination_bounds),
                height(destination_bounds),
                BLACKNESS,
            )
        };
    }
}
