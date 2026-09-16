//! Win32 equivalent of the small borderless `FrigoForm` base class.

use std::ptr::{NonNull, null, null_mut};
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, RECT, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, HTTRANSPARENT,
    HWND_TOPMOST, RegisterClassW, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_NOOWNERZORDER,
    SetWindowPos, ShowWindow, WM_ERASEBKGND, WM_NCHITTEST, WM_PAINT, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

const CLASS_NAME: &[u16] = &[
    b'F' as u16,
    b'r' as u16,
    b'i' as u16,
    b'g' as u16,
    b'o' as u16,
    b'T' as u16,
    b'a' as u16,
    b'b' as u16,
    b'W' as u16,
    b'i' as u16,
    b'n' as u16,
    b'd' as u16,
    b'o' as u16,
    b'w' as u16,
    0,
];

fn error(operation: &str) -> String {
    format!("{operation} failed (Win32 error {})", unsafe {
        GetLastError()
    })
}

fn register_class() -> Result<(), String> {
    static REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| {
            let instance = unsafe { GetModuleHandleW(null()) };
            if instance.is_null() {
                return Err(error("GetModuleHandleW"));
            }
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: null_mut(),
                hCursor: null_mut(),
                hbrBackground: null_mut(),
                lpszMenuName: null(),
                lpszClassName: CLASS_NAME.as_ptr(),
            };
            let atom = unsafe { RegisterClassW(&class) };
            if atom == 0 && unsafe { GetLastError() } != ERROR_CLASS_ALREADY_EXISTS {
                return Err(error("RegisterClassW"));
            }
            Ok(())
        })
        .clone()
}

/// Borderless, owner-owned popup for an application tile. It has no client
/// painting of its own; `LayerUpdater` supplies the layered surface.
pub struct FrigoWindow {
    hwnd: NonNull<std::ffi::c_void>,
}

impl FrigoWindow {
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is opaque, never dereferenced as Rust memory.
    pub fn new(owner: HWND, bounds: RECT) -> Result<Self, String> {
        register_class()?;
        let instance = unsafe { GetModuleHandleW(null()) };
        if instance.is_null() {
            return Err(error("GetModuleHandleW"));
        }
        let width = bounds.right - bounds.left;
        let height = bounds.bottom - bounds.top;
        if width <= 0 || height <= 0 {
            return Err("FrigoWindow requires positive bounds".to_string());
        }
        // FormBorderStyle=None + ShowInTaskbar=false + TopMost=true, plus the
        // exact styles added by ApplicationWindow.CreateParams.
        let ex_style =
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_NOACTIVATE;
        let raw = unsafe {
            CreateWindowExW(
                ex_style,
                CLASS_NAME.as_ptr(),
                CLASS_NAME.as_ptr(),
                WS_POPUP,
                bounds.left,
                bounds.top,
                width,
                height,
                owner,
                null_mut(),
                instance,
                null(),
            )
        };
        let Some(hwnd) = NonNull::new(raw) else {
            return Err(error("CreateWindowExW"));
        };
        // Take ownership before the final placement call. If SetWindowPos
        // fails, returning the error drops this guard and destroys the HWND
        // instead of leaking a partially-created popup.
        let window = Self { hwnd };
        unsafe {
            // HWND_TOPMOST is kept explicit to preserve the original
            // topmost-window behavior through SetWindowPos.
            if SetWindowPos(
                window.hwnd(),
                HWND_TOPMOST,
                bounds.left,
                bounds.top,
                width,
                height,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            ) == 0
            {
                return Err(error("SetWindowPos"));
            }
        }
        Ok(window)
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd.as_ptr()
    }

    pub fn set_visible(&self, visible: bool) {
        let command = if visible { SW_SHOW } else { SW_HIDE };
        unsafe {
            ShowWindow(self.hwnd(), command);
        }
    }

    fn destroy(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd());
        }
    }
}

impl Drop for FrigoWindow {
    fn drop(&mut self) {
        self.destroy();
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCHITTEST => HTTRANSPARENT as LRESULT,
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            unsafe {
                BeginPaint(hwnd, &mut paint);
                EndPaint(hwnd, &paint);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
