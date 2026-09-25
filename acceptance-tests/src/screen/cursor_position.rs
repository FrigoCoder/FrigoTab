use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

pub struct CursorPosition(POINT);

impl CursorPosition {
    pub fn capture() -> Option<Self> {
        let mut position = POINT::default();
        (unsafe { GetCursorPos(&mut position) } != 0).then_some(Self(position))
    }

    pub fn move_to(x: i32, y: i32) -> bool {
        unsafe { SetCursorPos(x, y) != 0 }
    }
}

impl Drop for CursorPosition {
    fn drop(&mut self) {
        unsafe { SetCursorPos(self.0.x, self.0.y) };
    }
}
