//! Native Explorer desktop snapshot used by the application.
//!
//! The source is Explorer's desktop view, never the screen DC.  This matters
//! because a screen capture would copy whichever application windows happen to
//! cover the wallpaper.  Explorer is rendered once into a retained top-down
//! DIB and that DIB is reused while the switcher is shown.

use std::cmp::{max, min};
use std::ffi::c_void;
use std::ptr::{NonNull, null, null_mut};

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BLACKNESS, BitBlt, COLORONCOLOR, ClientToScreen,
    CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDC, HDC, HGDIOBJ, PatBlt, ReleaseDC, SRCCOPY, SelectObject, SetStretchBltMode,
    StretchBlt,
};
use windows_sys::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetClientRect, GetShellWindow, PW_RENDERFULLCONTENT,
};
use windows_sys::core::BOOL;

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

/// Owns a memory DC and the bitmap selected into it.
///
/// GDI requires the selected bitmap to be restored before it is deleted.  The
/// guard also owns partially-created surfaces, so every failure after the DC
/// is created follows the same cleanup path as a successfully published
/// surface.
struct GdiMemorySurface {
    dc: GdiHandle,
    bitmap: Option<GdiHandle>,
    previous_bitmap: Option<GdiHandle>,
}

type GdiHandle = NonNull<c_void>;

impl GdiMemorySurface {
    fn new_dc(source_dc: HDC, name: &str) -> Result<Self, ()> {
        let dc = unsafe { CreateCompatibleDC(source_dc) };
        let Some(dc) = NonNull::new(dc) else {
            write_diagnostic(&format!("CreateCompatibleDC failed for the {name}"));
            return Err(());
        };
        Ok(Self {
            dc,
            bitmap: None,
            previous_bitmap: None,
        })
    }

    fn new_compatible(source_dc: HDC, width: i32, height: i32) -> Result<Self, ()> {
        let mut surface = Self::new_dc(source_dc, "shell print surface")?;
        surface.bitmap = NonNull::new(unsafe { CreateCompatibleBitmap(source_dc, width, height) });
        if surface.bitmap.is_none() {
            write_diagnostic("CreateCompatibleBitmap failed for the shell print surface");
            return Err(());
        }
        surface.select("shell print surface")
    }

    fn new_dib(source_dc: HDC, source_bounds: RECT) -> Result<Self, ()> {
        let mut surface = Self::new_dc(source_dc, "shell snapshot")?;
        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width(source_bounds),
                // A negative height creates a top-down DIB whose first
                // scan line is the top of the virtual desktop.
                biHeight: -height(source_bounds),
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = null_mut();
        surface.bitmap = NonNull::new(unsafe {
            CreateDIBSection(
                source_dc,
                &bitmap_info,
                DIB_RGB_COLORS,
                &mut bits,
                null_mut(),
                0,
            )
        });
        if surface.bitmap.is_none() {
            write_diagnostic("CreateDIBSection failed for the shell snapshot");
            return Err(());
        }
        if bits.is_null() {
            write_diagnostic("CreateDIBSection returned no writable shell-snapshot pixels");
            return Err(());
        }
        surface.select("shell snapshot")
    }

    fn select(mut self, name: &str) -> Result<Self, ()> {
        let Some(bitmap) = self.bitmap else {
            write_diagnostic(&format!("SelectObject failed for the {name}"));
            return Err(());
        };
        self.previous_bitmap =
            valid_gdi_handle(unsafe { SelectObject(self.dc(), bitmap.as_ptr()) });
        if self.previous_bitmap.is_none() {
            write_diagnostic(&format!("SelectObject failed for the {name}"));
            return Err(());
        }
        Ok(self)
    }

    fn dc(&self) -> HDC {
        self.dc.as_ptr()
    }
}

impl Drop for GdiMemorySurface {
    fn drop(&mut self) {
        let mut bitmap_deselected = true;
        if let Some(previous_bitmap) = self.previous_bitmap {
            let restored = unsafe { SelectObject(self.dc(), previous_bitmap.as_ptr()) };
            bitmap_deselected = !invalid_gdi_object(restored);
            if !bitmap_deselected {
                write_diagnostic("SelectObject failed while releasing the shell snapshot");
            }
        }
        self.previous_bitmap = None;

        if let Some(bitmap) = self.bitmap.take() {
            if bitmap_deselected {
                if unsafe { DeleteObject(bitmap.as_ptr()) } == 0 {
                    write_diagnostic("DeleteObject failed while releasing the shell snapshot");
                }
            } else {
                self.bitmap = Some(bitmap);
            }
        }
        if unsafe { DeleteDC(self.dc()) } == 0 {
            write_diagnostic("DeleteDC failed while releasing the shell snapshot");
        }
        if let Some(bitmap) = self.bitmap.take() {
            // A selected bitmap cannot be deleted until its memory DC is gone.
            // Destroying that DC above releases the selection so this retry
            // is safe and matches the original cleanup path.
            if unsafe { DeleteObject(bitmap.as_ptr()) } == 0 {
                write_diagnostic("DeleteObject failed after releasing the shell snapshot DC");
            }
        }
    }
}

/// A DC acquired from a window.  Unlike a memory DC, this handle must be
/// released with ReleaseDC and the owning HWND is therefore retained here.
struct WindowDc {
    hwnd: HWND,
    dc: GdiHandle,
}

impl WindowDc {
    fn new(hwnd: HWND) -> Result<Self, ()> {
        let dc = unsafe { GetDC(hwnd) };
        let Some(dc) = NonNull::new(dc) else {
            write_diagnostic("GetDC failed for the Explorer desktop host");
            return Err(());
        };
        Ok(Self { hwnd, dc })
    }

    fn dc(&self) -> HDC {
        self.dc.as_ptr()
    }
}

impl Drop for WindowDc {
    fn drop(&mut self) {
        if unsafe { ReleaseDC(self.hwnd, self.dc()) } == 0 {
            write_diagnostic("ReleaseDC failed for the Explorer desktop host");
        }
    }
}

struct GdiShellDesktopSnapshotFrame {
    surface: GdiMemorySurface,
    size: (i32, i32),
}

impl GdiShellDesktopSnapshotFrame {
    fn new(source_bounds: RECT) -> Result<Self, ()> {
        let shell_desktop = ShellDesktopWindowLocator::find();
        if shell_desktop.is_null() {
            write_diagnostic("The Explorer desktop host is unavailable");
            return Err(());
        }

        let source_dc = WindowDc::new(shell_desktop)?;
        Self::new_with_source_dc(shell_desktop, source_dc.dc(), source_bounds)
    }

    fn new_with_source_dc(
        shell_desktop: HWND,
        source_dc: HDC,
        source_bounds: RECT,
    ) -> Result<Self, ()> {
        let mut client_rect = RECT::default();
        if unsafe { GetClientRect(shell_desktop, &mut client_rect) } == 0 {
            write_diagnostic("GetClientRect failed for the Explorer desktop host");
            return Err(());
        }
        if width(client_rect) <= 0 || height(client_rect) <= 0 {
            write_diagnostic("The Explorer desktop host has stale geometry");
            return Err(());
        }

        let mut client_origin = POINT { x: 0, y: 0 };
        if unsafe { ClientToScreen(shell_desktop, &mut client_origin) } == 0 {
            write_diagnostic("ClientToScreen failed for the Explorer desktop host");
            return Err(());
        }

        let desktop_client_bounds = RECT {
            left: client_origin.x,
            top: client_origin.y,
            right: client_origin.x + width(client_rect),
            bottom: client_origin.y + height(client_rect),
        };
        let Some(copy_bounds) = intersect(source_bounds, desktop_client_bounds) else {
            write_diagnostic("The Explorer desktop host does not intersect the virtual desktop");
            return Err(());
        };

        let memory_surface = GdiMemorySurface::new_dib(source_dc, source_bounds)?;

        // Initialize every pixel to black.  The shell host can be smaller
        // than a disconnected monitor or report stale bounds during an
        // Explorer restart; uncovered portions must never come from a
        // screen DC.
        if unsafe {
            PatBlt(
                memory_surface.dc(),
                0,
                0,
                width(source_bounds),
                height(source_bounds),
                BLACKNESS,
            )
        } == 0
        {
            write_diagnostic("PatBlt failed for the shell snapshot");
            return Err(());
        }

        let source_x = copy_bounds.left - desktop_client_bounds.left;
        let source_y = copy_bounds.top - desktop_client_bounds.top;
        let destination_x = copy_bounds.left - source_bounds.left;
        let destination_y = copy_bounds.top - source_bounds.top;

        // PrintWindow(PW_RENDERFULLCONTENT) asks Explorer to render its
        // desktop host and icon list into an off-screen surface.  A plain
        // window DC, DWM thumbnail, or screen capture is intentionally
        // not used because each can include covering applications.
        let printed_surface =
            GdiMemorySurface::new_compatible(source_dc, width(client_rect), height(client_rect))?;
        if unsafe {
            PatBlt(
                printed_surface.dc(),
                0,
                0,
                width(client_rect),
                height(client_rect),
                BLACKNESS,
            )
        } == 0
        {
            write_diagnostic("PatBlt failed for the shell print surface");
            return Err(());
        }
        if print_window(shell_desktop, printed_surface.dc(), PW_RENDERFULLCONTENT) == 0 {
            write_diagnostic("PrintWindow failed for the Explorer desktop host");
            return Err(());
        }
        if unsafe {
            BitBlt(
                memory_surface.dc(),
                destination_x,
                destination_y,
                width(copy_bounds),
                height(copy_bounds),
                printed_surface.dc(),
                source_x,
                source_y,
                SRCCOPY,
            )
        } == 0
        {
            write_diagnostic("BitBlt failed while mapping the shell print surface");
            return Err(());
        }

        Ok(Self {
            surface: memory_surface,
            size: (width(source_bounds), height(source_bounds)),
        })
    }

    fn draw(&self, destination_dc: HDC, destination_bounds: RECT) -> Result<(), ()> {
        let (source_width, source_height) = self.size;
        let painted = if destination_bounds.right - destination_bounds.left == source_width
            && destination_bounds.bottom - destination_bounds.top == source_height
        {
            unsafe {
                BitBlt(
                    destination_dc,
                    destination_bounds.left,
                    destination_bounds.top,
                    source_width,
                    source_height,
                    self.surface.dc(),
                    0,
                    0,
                    SRCCOPY,
                )
            }
        } else {
            let previous_mode = unsafe { SetStretchBltMode(destination_dc, COLORONCOLOR) };
            let painted = unsafe {
                StretchBlt(
                    destination_dc,
                    destination_bounds.left,
                    destination_bounds.top,
                    width(destination_bounds),
                    height(destination_bounds),
                    self.surface.dc(),
                    0,
                    0,
                    source_width,
                    source_height,
                    SRCCOPY,
                )
            };
            if previous_mode != 0 {
                unsafe { SetStretchBltMode(destination_dc, previous_mode) };
            }
            painted
        };
        if painted == 0 { Err(()) } else { Ok(()) }
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

fn width(rectangle: RECT) -> i32 {
    rectangle.right - rectangle.left
}

fn height(rectangle: RECT) -> i32 {
    rectangle.bottom - rectangle.top
}

fn intersect(first: RECT, second: RECT) -> Option<RECT> {
    let result = RECT {
        left: max(first.left, second.left),
        top: max(first.top, second.top),
        right: min(first.right, second.right),
        bottom: min(first.bottom, second.bottom),
    };
    (width(result) > 0 && height(result) > 0).then_some(result)
}

fn invalid_gdi_object(value: HGDIOBJ) -> bool {
    value.is_null() || value == (-1isize as *mut c_void)
}

fn valid_gdi_handle(value: HGDIOBJ) -> Option<GdiHandle> {
    if invalid_gdi_object(value) {
        None
    } else {
        NonNull::new(value)
    }
}

fn write_diagnostic(message: &str) {
    let message = format!("{message}\0");
    let wide: Vec<u16> = message.encode_utf16().collect();
    unsafe { OutputDebugStringW(wide.as_ptr()) };
}

struct ShellDesktopWindowLocator;

const fn wide_class<const N: usize>(bytes: &[u8; N]) -> [u16; N] {
    let mut wide = [0; N];
    let mut index = 0;
    while index < N {
        wide[index] = bytes[index] as u16;
        index += 1;
    }
    wide
}

const WORKERW_CLASS: [u16; 8] = wide_class(b"WorkerW\0");
const DESKTOP_VIEW_CLASS: [u16; 17] = wide_class(b"SHELLDLL_DefView\0");
const ICON_VIEW_CLASS: [u16; 14] = wide_class(b"SysListView32\0");

impl ShellDesktopWindowLocator {
    fn find() -> HWND {
        let shell = unsafe { GetShellWindow() };
        let desktop_view = Self::find_desktop_view(shell);
        if !desktop_view.is_null() {
            return shell;
        }

        let mut worker = null_mut();
        loop {
            worker = unsafe { FindWindowExW(null_mut(), worker, WORKERW_CLASS.as_ptr(), null()) };
            if worker.is_null() {
                return null_mut();
            }
            if !Self::find_desktop_view(worker).is_null() {
                return worker;
            }
        }
    }

    fn find_desktop_view(parent: HWND) -> HWND {
        if parent.is_null() {
            return null_mut();
        }
        let desktop_view =
            unsafe { FindWindowExW(parent, null_mut(), DESKTOP_VIEW_CLASS.as_ptr(), null()) };
        if desktop_view.is_null() {
            return null_mut();
        }

        // Require Explorer's icon list so an unrelated intermediate window
        // with the same class cannot become a capture source.
        let icon_view =
            unsafe { FindWindowExW(desktop_view, null_mut(), ICON_VIEW_CLASS.as_ptr(), null()) };
        if icon_view.is_null() {
            null_mut()
        } else {
            desktop_view
        }
    }
}

// windows-sys gates PrintWindow behind Win32_Storage_Xps.  Keep this one
// declaration local so the desktop capture remains available with the small
// feature set used by the application.
#[link(name = "user32")]
unsafe extern "system" {
    fn PrintWindow(hwnd: HWND, hdc_blt: HDC, flags: u32) -> BOOL;
}

fn print_window(hwnd: HWND, hdc: HDC, flags: u32) -> BOOL {
    unsafe { PrintWindow(hwnd, hdc, flags) }
}
