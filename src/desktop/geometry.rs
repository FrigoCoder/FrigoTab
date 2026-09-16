use std::cmp::{max, min};

use windows_sys::Win32::Foundation::RECT;

pub(super) fn width(rectangle: RECT) -> i32 {
    rectangle.right - rectangle.left
}

pub(super) fn height(rectangle: RECT) -> i32 {
    rectangle.bottom - rectangle.top
}

pub(super) fn intersect(first: RECT, second: RECT) -> Option<RECT> {
    let result = RECT {
        left: max(first.left, second.left),
        top: max(first.top, second.top),
        right: min(first.right, second.right),
        bottom: min(first.bottom, second.bottom),
    };
    (width(result) > 0 && height(result) > 0).then_some(result)
}
