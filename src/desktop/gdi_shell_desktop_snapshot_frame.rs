use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BLACKNESS, BitBlt, COLORONCOLOR, ClientToScreen, HDC, PatBlt, SRCCOPY, SetStretchBltMode,
    StretchBlt,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, PW_RENDERFULLCONTENT};
use windows_sys::core::BOOL;

use super::diagnostics::write_diagnostic;
use super::gdi_memory_surface::GdiMemorySurface;
use super::geometry::{height, intersect, width};
use super::shell_desktop_window_locator::ShellDesktopWindowLocator;
use super::window_dc::WindowDc;

pub(super) struct GdiShellDesktopSnapshotFrame {
    surface: GdiMemorySurface,
    size: (i32, i32),
}

impl GdiShellDesktopSnapshotFrame {
    pub(super) fn new(source_bounds: RECT) -> Result<Self, ()> {
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

    pub(super) fn draw(&self, destination_dc: HDC, destination_bounds: RECT) -> Result<(), ()> {
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
