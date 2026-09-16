use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

pub(crate) struct CursorPosition(POINT);

impl CursorPosition {
    pub(crate) fn capture() -> Option<Self> {
        let mut position = POINT::default();
        (unsafe { GetCursorPos(&mut position) } != 0).then_some(Self(position))
    }
}

impl Drop for CursorPosition {
    fn drop(&mut self) {
        unsafe { SetCursorPos(self.0.x, self.0.y) };
    }
}
