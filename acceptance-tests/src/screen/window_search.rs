use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GWL_STYLE, GetForegroundWindow, GetWindow,
    GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindow, IsWindowVisible, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
};

#[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque test-fixture handle.
pub fn window_bounds(hwnd: HWND) -> Option<RECT> {
    if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
        return None;
    }
    let mut bounds = RECT::default();
    (unsafe { GetWindowRect(hwnd, &mut bounds) } != 0
        && bounds.right > bounds.left
        && bounds.bottom > bounds.top)
        .then_some(bounds)
}
#[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque test-fixture handle.
pub fn get_window_rect(hwnd: HWND) -> Option<RECT> {
    window_bounds(hwnd)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque test-fixture handle.
pub fn window_title(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    unsafe {
        let length = GetWindowTextLengthW(hwnd);
        if length <= 0 {
            return String::new();
        }
        let mut text = vec![0u16; length as usize + 1];
        let copied = GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32);
        String::from_utf16_lossy(&text[..copied.max(0) as usize])
    }
}
#[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque test-fixture handle.
pub fn is_window_visible(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsWindowVisible(hwnd) != 0 }
}
#[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque test-fixture handle.
pub fn window_style(hwnd: HWND) -> u32 {
    unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 }
}
#[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque test-fixture handle.
pub fn window_ex_style(hwnd: HWND) -> u32 {
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 }
}
#[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque test-fixture handle.
pub fn window_owner(hwnd: HWND) -> HWND {
    unsafe { GetWindow(hwnd, GW_OWNER) }
}
pub fn foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}
pub fn is_layered(hwnd: HWND) -> bool {
    window_ex_style(hwnd) & WS_EX_LAYERED != 0
}

pub fn windows_for_pid(pid: u32) -> Vec<HWND> {
    let mut search = WindowSearch {
        pid,
        result: Vec::new(),
    };
    unsafe { EnumWindows(Some(enum_windows_callback), &mut search as *mut _ as LPARAM) };
    search.result
}
pub fn enumerate_windows(pid: u32) -> Vec<HWND> {
    windows_for_pid(pid)
}

pub fn visible_owned_layered_windows(owner: HWND) -> Vec<HWND> {
    let pid = window_pid(owner);
    windows_for_pid(pid)
        .into_iter()
        .filter(|&hwnd| {
            hwnd != owner
                && is_window_visible(hwnd)
                && is_layered(hwnd)
                && window_owner(hwnd) == owner
        })
        .collect()
}
pub fn visible_owned_overlays(owner: HWND) -> Vec<HWND> {
    visible_owned_layered_windows(owner)
}

pub fn find_window_by_pid_and_bounds(
    pid: u32,
    minimum_width: i32,
    minimum_height: i32,
) -> Option<HWND> {
    windows_for_pid(pid).into_iter().find(|&hwnd| {
        window_bounds(hwnd).is_some_and(|bounds| {
            bounds.right - bounds.left >= minimum_width
                && bounds.bottom - bounds.top >= minimum_height
        })
    })
}

struct WindowSearch {
    pid: u32,
    result: Vec<HWND>,
}
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> i32 {
    let search = unsafe { &mut *(lparam as *mut WindowSearch) };
    if window_pid(hwnd) == search.pid {
        search.result.push(hwnd);
    }
    1
}
fn window_pid(hwnd: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    pid
}

pub(crate) fn find_owner(pid: u32) -> Option<HWND> {
    windows_for_pid(pid).into_iter().find(|&hwnd| {
        window_ex_style(hwnd) & WS_EX_TOOLWINDOW != 0
            && window_bounds(hwnd).is_some_and(|bounds| {
                bounds.right - bounds.left >= 100 && bounds.bottom - bounds.top >= 100
            })
    })
}
