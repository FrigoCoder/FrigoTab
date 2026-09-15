//! Native Explorer desktop snapshot used by the application.
//!
//! The source is Explorer's desktop view, never the screen DC.  This matters
//! because a screen capture would copy whichever application windows happen to
//! cover the wallpaper.  Explorer is rendered once into a retained top-down
//! DIB and that DIB is reused while the switcher is shown.

use std::cmp::{max, min};
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BLACKNESS, BitBlt, COLORONCOLOR, ClientToScreen,
    CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDC, HBITMAP, HDC, HGDIOBJ, PatBlt, ReleaseDC, SRCCOPY, SelectObject,
    SetStretchBltMode, StretchBlt,
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

    /// Alias matching the original type's constructor-shaped call site.
    pub fn new(source_bounds: RECT) -> Self {
        Self::capture(source_bounds)
    }

    pub fn is_available(&self) -> bool {
        self.frame.is_some()
    }

    /// Size of the captured virtual desktop, or `(0, 0)` for an unavailable
    /// snapshot.
    pub fn size(&self) -> (i32, i32) {
        self.frame
            .as_ref()
            .map_or((0, 0), GdiShellDesktopSnapshotFrame::size)
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

struct GdiShellDesktopSnapshotFrame {
    memory_dc: HDC,
    bitmap: HBITMAP,
    previous_bitmap: HGDIOBJ,
    size: (i32, i32),
}

impl GdiShellDesktopSnapshotFrame {
    fn new(source_bounds: RECT) -> Result<Self, ()> {
        let shell_desktop = ShellDesktopWindowLocator::find();
        if shell_desktop.is_null() {
            write_diagnostic("The Explorer desktop host is unavailable");
            return Err(());
        }

        let source_dc = unsafe { GetDC(shell_desktop) };
        if source_dc.is_null() {
            write_diagnostic("GetDC failed for the Explorer desktop host");
            return Err(());
        }

        let result = Self::new_with_source_dc(shell_desktop, source_dc, source_bounds);
        let released = unsafe { ReleaseDC(shell_desktop, source_dc) };
        if released == 0 {
            write_diagnostic("ReleaseDC failed for the Explorer desktop host");
        }
        result
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

        let mut memory_dc: HDC = null_mut();
        let mut bitmap: HBITMAP = null_mut();
        let mut previous_bitmap: HGDIOBJ = null_mut();
        let mut committed = false;
        let result = (|| {
            memory_dc = unsafe { CreateCompatibleDC(source_dc) };
            if memory_dc.is_null() {
                write_diagnostic("CreateCompatibleDC failed for the shell snapshot");
                return Err(());
            }

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
            bitmap = unsafe {
                CreateDIBSection(
                    source_dc,
                    &bitmap_info,
                    DIB_RGB_COLORS,
                    &mut bits,
                    null_mut(),
                    0,
                )
            };
            if bitmap.is_null() {
                write_diagnostic("CreateDIBSection failed for the shell snapshot");
                return Err(());
            }
            if bits.is_null() {
                write_diagnostic("CreateDIBSection returned no writable shell-snapshot pixels");
                return Err(());
            }

            previous_bitmap = unsafe { SelectObject(memory_dc, bitmap as HGDIOBJ) };
            if invalid_gdi_object(previous_bitmap) {
                write_diagnostic("SelectObject failed for the shell snapshot");
                return Err(());
            }

            // Initialize every pixel to black.  The shell host can be smaller
            // than a disconnected monitor or report stale bounds during an
            // Explorer restart; uncovered portions must never come from a
            // screen DC.
            if unsafe {
                PatBlt(
                    memory_dc,
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
            let mut printed_dc: HDC = null_mut();
            let mut printed_bitmap: HBITMAP = null_mut();
            let mut printed_previous_bitmap: HGDIOBJ = null_mut();
            let printed_result = (|| {
                printed_dc = unsafe { CreateCompatibleDC(source_dc) };
                if printed_dc.is_null() {
                    write_diagnostic("CreateCompatibleDC failed for the shell print surface");
                    return Err(());
                }
                printed_bitmap = unsafe {
                    CreateCompatibleBitmap(source_dc, width(client_rect), height(client_rect))
                };
                if printed_bitmap.is_null() {
                    write_diagnostic("CreateCompatibleBitmap failed for the shell print surface");
                    return Err(());
                }
                printed_previous_bitmap =
                    unsafe { SelectObject(printed_dc, printed_bitmap as HGDIOBJ) };
                if invalid_gdi_object(printed_previous_bitmap) {
                    write_diagnostic("SelectObject failed for the shell print surface");
                    return Err(());
                }
                if unsafe {
                    PatBlt(
                        printed_dc,
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
                if unsafe { print_window(shell_desktop, printed_dc, PW_RENDERFULLCONTENT) } == 0 {
                    write_diagnostic("PrintWindow failed for the Explorer desktop host");
                    return Err(());
                }
                if unsafe {
                    BitBlt(
                        memory_dc,
                        destination_x,
                        destination_y,
                        width(copy_bounds),
                        height(copy_bounds),
                        printed_dc,
                        source_x,
                        source_y,
                        SRCCOPY,
                    )
                } == 0
                {
                    write_diagnostic("BitBlt failed while mapping the shell print surface");
                    return Err(());
                }
                Ok(())
            })();
            release_surface(
                &mut printed_dc,
                &mut printed_bitmap,
                &mut printed_previous_bitmap,
            );
            printed_result?;

            committed = true;
            Ok(Self {
                memory_dc,
                bitmap,
                previous_bitmap,
                size: (width(source_bounds), height(source_bounds)),
            })
        })();

        if !committed {
            release_surface(&mut memory_dc, &mut bitmap, &mut previous_bitmap);
        }
        result
    }

    fn size(&self) -> (i32, i32) {
        self.size
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
                    self.memory_dc,
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
                    self.memory_dc,
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

impl Drop for GdiShellDesktopSnapshotFrame {
    fn drop(&mut self) {
        release_surface(
            &mut self.memory_dc,
            &mut self.bitmap,
            &mut self.previous_bitmap,
        );
    }
}

fn release_surface(dc: &mut HDC, bitmap: &mut HBITMAP, previous_bitmap: &mut HGDIOBJ) {
    let mut bitmap_deselected = true;
    if !dc.is_null() && !previous_bitmap.is_null() && !invalid_gdi_object(*previous_bitmap) {
        let restored = unsafe { SelectObject(*dc, *previous_bitmap) };
        bitmap_deselected = !invalid_gdi_object(restored);
        if !bitmap_deselected {
            write_diagnostic("SelectObject failed while releasing the shell snapshot");
        }
    }
    *previous_bitmap = null_mut();

    if !bitmap.is_null() && bitmap_deselected {
        if unsafe { DeleteObject(*bitmap as HGDIOBJ) } == 0 {
            write_diagnostic("DeleteObject failed while releasing the shell snapshot");
        }
        *bitmap = null_mut();
    }
    if !dc.is_null() {
        if unsafe { DeleteDC(*dc) } == 0 {
            write_diagnostic("DeleteDC failed while releasing the shell snapshot");
        }
        *dc = null_mut();
    }
    if !bitmap.is_null() {
        // A selected bitmap cannot be deleted until its memory DC is gone.
        // Destroying that DC above releases the selection so this retry is
        // safe and matches the original cleanup path.
        if unsafe { DeleteObject(*bitmap as HGDIOBJ) } == 0 {
            write_diagnostic("DeleteObject failed after releasing the shell snapshot DC");
        }
        *bitmap = null_mut();
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

fn write_diagnostic(message: &str) {
    let message = format!("{message}\0");
    let wide: Vec<u16> = message.encode_utf16().collect();
    unsafe { OutputDebugStringW(wide.as_ptr()) };
}

struct ShellDesktopWindowLocator;

impl ShellDesktopWindowLocator {
    fn find() -> HWND {
        let shell = unsafe { GetShellWindow() };
        let desktop_view = Self::find_desktop_view(shell);
        if !desktop_view.is_null() {
            return shell;
        }

        let mut worker = null_mut();
        loop {
            worker = unsafe {
                FindWindowExW(null_mut(), worker, class_name("WorkerW").as_ptr(), null())
            };
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
        let desktop_view = unsafe {
            FindWindowExW(
                parent,
                null_mut(),
                class_name("SHELLDLL_DefView").as_ptr(),
                null(),
            )
        };
        if desktop_view.is_null() {
            return null_mut();
        }

        // Require Explorer's icon list so an unrelated intermediate window
        // with the same class cannot become a capture source.
        let icon_view = unsafe {
            FindWindowExW(
                desktop_view,
                null_mut(),
                class_name("SysListView32").as_ptr(),
                null(),
            )
        };
        if icon_view.is_null() {
            null_mut()
        } else {
            desktop_view
        }
    }
}

fn class_name(name: &str) -> Vec<u16> {
    name.encode_utf16().chain(std::iter::once(0)).collect()
}

// windows-sys gates PrintWindow behind Win32_Storage_Xps.  Keep this one
// declaration local so the desktop capture remains available with the small
// feature set used by the application.
#[link(name = "user32")]
unsafe extern "system" {
    fn PrintWindow(hwnd: HWND, hdc_blt: HDC, flags: u32) -> BOOL;
}

unsafe fn print_window(hwnd: HWND, hdc: HDC, flags: u32) -> BOOL {
    unsafe { PrintWindow(hwnd, hdc, flags) }
}
