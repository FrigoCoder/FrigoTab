use std::io;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus};
use std::ptr::null_mut;
use std::thread;
use std::time::{Duration, Instant};

use frigotab::sys_tray_icon::TRAY_CALLBACK_MESSAGE;
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT, SendInput,
    VK_ESCAPE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetMenuItemCount, GetMenuItemInfoW, GetMenuItemRect, GetSubMenu, HMENU,
    IsWindow, MENUITEMINFOW, MFS_CHECKED, MIIM_FTYPE, MIIM_STATE, MIIM_STRING, MIIM_SUBMENU,
    MN_GETHMENU, PostMessageW, SendMessageW, SetCursorPos, WM_CLOSE, WM_KEYDOWN, WM_KEYUP,
    WM_RBUTTONUP,
};

use super::tray_menu_item::TrayMenuItem;
use crate::screen::cursor_position::CursorPosition;
use crate::screen::window_search::{
    find_owner, visible_owned_layered_windows, window_bounds, windows_for_pid,
};
use crate::{is_window_visible, pump_messages, set_per_monitor_dpi_awareness, wait_until};

pub const WM_BEGIN_SESSION: u32 = 0x4001;

pub struct RunningFrigoTab {
    process: Child,
    owner: HWND,
}

impl RunningFrigoTab {
    pub fn start() -> io::Result<Self> {
        set_per_monitor_dpi_awareness();
        let mut process = Command::new(executable_path()?).spawn()?;
        let pid = process.id();
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            pump_messages();
            if let Some(owner) = find_owner(pid) {
                return Ok(Self { process, owner });
            }
            if let Some(status) = process.try_wait()? {
                return Err(io::Error::other(format!(
                    "FrigoTab exited before creating its owner window ({status})"
                )));
            }
            if Instant::now() >= deadline {
                let _ = process.kill();
                let _ = process.wait();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "FrigoTab owner was not created",
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn executable_path() -> io::Result<PathBuf> {
        executable_path()
    }

    pub fn owner(&self) -> HWND {
        self.owner
    }
    pub fn session_window(&self) -> HWND {
        self.owner
    }
    pub fn pid(&self) -> u32 {
        self.process.id()
    }
    pub fn windows(&self) -> Vec<HWND> {
        windows_for_pid(self.pid())
    }
    pub fn visible_windows(&self) -> Vec<HWND> {
        self.windows()
            .into_iter()
            .filter(|&h| is_window_visible(h))
            .collect()
    }
    pub fn visible_owned_layered_windows(&self) -> Vec<HWND> {
        visible_owned_layered_windows(self.owner)
    }
    pub fn visible_owned_overlays(&self) -> Vec<HWND> {
        self.visible_owned_layered_windows()
    }

    pub fn has_exited(&mut self) -> io::Result<bool> {
        Ok(self.process.try_wait()?.is_some())
    }
    pub fn exit_code(&mut self) -> io::Result<Option<i32>> {
        Ok(self.process.try_wait()?.and_then(|s| s.code()))
    }

    pub fn open(&self) -> bool {
        self.post(WM_BEGIN_SESSION, 0)
    }
    pub fn begin_session(&self) -> bool {
        self.open()
    }
    pub fn is_visible(&self) -> bool {
        is_window_visible(self.owner)
    }
    pub fn visible(&self) -> bool {
        self.is_visible()
    }
    pub fn wait_visible(&self, timeout: Duration) -> bool {
        wait_until(timeout, || self.is_visible())
    }
    pub fn wait_hidden(&self, timeout: Duration) -> bool {
        wait_until(timeout, || !self.is_visible())
    }
    pub fn bounds(&self) -> RECT {
        window_bounds(self.owner).unwrap_or_default()
    }

    /// Opens and reads the actual tray popup, then dismisses it with Escape.
    /// This crosses the same owner-window callback and `TrackPopupMenu` path
    /// that a real notification-area right click uses.
    pub fn tray_menu(&self) -> Option<Vec<TrayMenuItem>> {
        let popup = self.open_tray_popup()?;
        let result = tray_menu_items(popup);
        dismiss_tray_popup(self.pid(), popup);
        result
    }

    /// Selects a child item by its visible labels in the real tray menu.
    /// `parent` names a submenu such as `Background`; `child` names its item
    /// such as `Black rectangle`.
    pub fn select_tray_menu_item(&self, parent: &str, child: &str) -> bool {
        let _cursor = CursorPosition::capture();
        let Some(popup) = self.open_tray_popup() else {
            return false;
        };
        let Some(menu) = popup_menu(popup) else {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        };
        let Some(items) = read_menu_items(menu, 0) else {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        };
        let Some(parent_index) = items
            .iter()
            .position(|item| item.label == parent && !item.children.is_empty())
        else {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        };
        let Some(child_index) = items[parent_index]
            .children
            .iter()
            .position(|item| item.label == child)
        else {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        };

        let submenu = unsafe { GetSubMenu(menu, parent_index as i32) };
        if submenu.is_null() || !click_menu_item(menu, parent_index) {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        }

        if !wait_until(Duration::from_secs(2), || {
            menu_item_rect(submenu, child_index).is_some()
        }) {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        }
        if !click_menu_item(submenu, child_index) {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        }

        wait_until(Duration::from_secs(2), || {
            find_tray_popup(self.pid()).is_none()
        })
    }

    /// Selects a top-level item by its visible label in the real tray menu.
    /// This is used for settings that do not need a submenu, such as the
    /// close-button visibility switch.
    pub fn select_top_level_tray_item(&self, label: &str) -> bool {
        let _cursor = CursorPosition::capture();
        let Some(popup) = self.open_tray_popup() else {
            return false;
        };
        let Some(menu) = popup_menu(popup) else {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        };
        let Some(items) = read_menu_items(menu, 0) else {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        };
        let Some(position) = items.iter().position(|item| item.label == label) else {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        };
        if !click_menu_item(menu, position) {
            dismiss_tray_popup(self.pid(), popup);
            return false;
        }
        wait_until(Duration::from_secs(2), || {
            find_tray_popup(self.pid()).is_none()
        })
    }

    fn open_tray_popup(&self) -> Option<HWND> {
        if self.owner.is_null()
            || unsafe { PostMessageW(self.owner, TRAY_CALLBACK_MESSAGE, 0, WM_RBUTTONUP as isize) }
                == 0
        {
            return None;
        }

        let mut popup = None;
        if !wait_until(Duration::from_secs(2), || {
            popup = find_tray_popup(self.pid());
            popup.is_some()
        }) {
            return None;
        }
        popup
    }

    pub fn wait_exit(&mut self, timeout: Duration) -> Option<ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(Some(status)) = self.process.try_wait() {
                return Some(status);
            }
            if Instant::now() >= deadline {
                return None;
            }
            pump_messages();
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn post(&self, message: u32, value: usize) -> bool {
        !self.owner.is_null() && unsafe { PostMessageW(self.owner, message, value, 0) != 0 }
    }
}

impl Drop for RunningFrigoTab {
    fn drop(&mut self) {
        if !self.owner.is_null() && unsafe { IsWindow(self.owner) } != 0 {
            unsafe { PostMessageW(self.owner, WM_CLOSE, 0, 0) };
        }
        if self.wait_exit(Duration::from_secs(2)).is_none() {
            let _ = self.process.kill();
            let _ = self.process.wait();
        }
    }
}

fn executable_path() -> io::Result<PathBuf> {
    if let Some(configured) = std::env::var_os("FRIGOTAB_EXE") {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Ok(path);
        }
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("FRIGOTAB_EXE is not a file: {}", path.display()),
        ));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        root.join("target/release/FrigoTab.exe"),
        root.join("../target/release/FrigoTab.exe"),
        root.join("target/debug/FrigoTab.exe"),
        root.join("../target/debug/FrigoTab.exe"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "FrigoTab.exe was not built"))
}

fn find_tray_popup(pid: u32) -> Option<HWND> {
    windows_for_pid(pid)
        .into_iter()
        .find(|&hwnd| is_window_visible(hwnd) && window_class(hwnd) == "#32768")
}

fn window_class(hwnd: HWND) -> String {
    let mut value = [0u16; 256];
    let length = unsafe { GetClassNameW(hwnd, value.as_mut_ptr(), value.len() as i32) };
    if length <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&value[..length as usize])
    }
}

fn tray_menu_items(popup: HWND) -> Option<Vec<TrayMenuItem>> {
    read_menu_items(popup_menu(popup)?, 0)
}

fn popup_menu(popup: HWND) -> Option<HMENU> {
    (!popup.is_null())
        .then(|| unsafe { SendMessageW(popup, MN_GETHMENU, 0, 0) as HMENU })
        .filter(|menu| !menu.is_null())
}

fn read_menu_items(menu: HMENU, depth: usize) -> Option<Vec<TrayMenuItem>> {
    if menu.is_null() || depth > 4 {
        return None;
    }
    let count = unsafe { GetMenuItemCount(menu) };
    if !(0..=32).contains(&count) {
        return None;
    }

    let mut items = Vec::with_capacity(count as usize);
    for position in 0..count {
        let mut text = [0u16; 128];
        let mut info = MENUITEMINFOW {
            cbSize: size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_FTYPE | MIIM_STATE | MIIM_STRING | MIIM_SUBMENU,
            dwTypeData: text.as_mut_ptr(),
            cch: (text.len() - 1) as u32,
            ..Default::default()
        };
        if unsafe { GetMenuItemInfoW(menu, position as u32, 1, &mut info) } == 0 {
            return None;
        }
        let length = text
            .iter()
            .position(|&character| character == 0)
            .unwrap_or(text.len());
        let children = if info.hSubMenu.is_null() {
            Vec::new()
        } else {
            read_menu_items(info.hSubMenu, depth + 1)?
        };
        items.push(TrayMenuItem {
            label: String::from_utf16_lossy(&text[..length]),
            checked: info.fState & MFS_CHECKED != 0,
            children,
        });
    }
    Some(items)
}

fn post_menu_key(popup: HWND, key: u16) -> bool {
    const KEY_UP_LPARAM: isize = (1 << 30) | (1 << 31) | 1;
    unsafe {
        SendMessageW(popup, WM_KEYDOWN, key as usize, 1);
        SendMessageW(popup, WM_KEYUP, key as usize, KEY_UP_LPARAM);
    }
    thread::sleep(Duration::from_millis(20));
    true
}

fn menu_item_rect(menu: HMENU, position: usize) -> Option<RECT> {
    if menu.is_null() {
        return None;
    }
    let mut bounds = RECT::default();
    (unsafe { GetMenuItemRect(null_mut(), menu, position as u32, &mut bounds) } != 0)
        .then_some(bounds)
}

fn click_menu_item(menu: HMENU, position: usize) -> bool {
    let Some(bounds) = menu_item_rect(menu, position) else {
        return false;
    };
    let x = bounds.left + (bounds.right - bounds.left) / 2;
    let y = bounds.top + (bounds.bottom - bounds.top) / 2;
    if unsafe { SetCursorPos(x, y) } == 0 {
        return false;
    }
    let inputs = [
        mouse_input(MOUSEEVENTF_LEFTDOWN),
        mouse_input(MOUSEEVENTF_LEFTUP),
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    thread::sleep(Duration::from_millis(40));
    sent == inputs.len() as u32
}

fn mouse_input(flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

fn dismiss_tray_popup(pid: u32, popup: HWND) {
    if !popup.is_null() {
        for _ in 0..4 {
            if find_tray_popup(pid).is_none() {
                return;
            }
            let _ = post_menu_key(popup, VK_ESCAPE);
        }
        let _ = wait_until(Duration::from_secs(2), || find_tray_popup(pid).is_none());
    }
}
