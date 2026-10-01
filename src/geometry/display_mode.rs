//! Read-only display-mode classification used around fullscreen transitions.

use std::mem::size_of;

use windows_sys::Win32::Graphics::Gdi::{
    DEVMODEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_MIRRORING_DRIVER, DISPLAY_DEVICEW,
    DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS,
    ENUM_REGISTRY_SETTINGS, EnumDisplayDevicesW, EnumDisplaySettingsExW,
};

/// Compare every active display's current geometry with the geometry persisted
/// by Windows. A mismatch is strong evidence of a temporary exclusive mode;
/// failures remain unknown because some remote/virtual display drivers do not
/// implement both queries.
pub fn current_display_geometry_is_persisted() -> Option<bool> {
    let mut index = 0;
    let mut found = false;
    loop {
        let mut device = DISPLAY_DEVICEW {
            cb: size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        if unsafe { EnumDisplayDevicesW(std::ptr::null(), index, &mut device, 0) } == 0 {
            break;
        }
        index += 1;
        if device.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0
            || device.StateFlags & DISPLAY_DEVICE_MIRRORING_DRIVER != 0
        {
            continue;
        }
        found = true;
        let current = display_mode(device.DeviceName.as_ptr(), ENUM_CURRENT_SETTINGS)?;
        let persisted = display_mode(device.DeviceName.as_ptr(), ENUM_REGISTRY_SETTINGS)?;
        if !same_geometry(current, persisted)? {
            return Some(false);
        }
    }
    found.then_some(true)
}

fn display_mode(device: *const u16, mode: u32) -> Option<DEVMODEW> {
    let mut result = DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    (unsafe { EnumDisplaySettingsExW(device, mode, &mut result, 0) } != 0).then_some(result)
}

fn same_geometry(current: DEVMODEW, persisted: DEVMODEW) -> Option<bool> {
    let common = current.dmFields & persisted.dmFields;
    if (common & (DM_PELSWIDTH | DM_PELSHEIGHT)) != (DM_PELSWIDTH | DM_PELSHEIGHT) {
        return None;
    }
    if common & DM_PELSWIDTH != 0 && current.dmPelsWidth != persisted.dmPelsWidth {
        return Some(false);
    }
    if common & DM_PELSHEIGHT != 0 && current.dmPelsHeight != persisted.dmPelsHeight {
        return Some(false);
    }
    if common & DM_DISPLAYORIENTATION != 0 {
        // SAFETY: Both DEVMODEW values advertise DM_DISPLAYORIENTATION.
        let (current_orientation, persisted_orientation) = unsafe {
            (
                current.Anonymous1.Anonymous2.dmDisplayOrientation,
                persisted.Anonymous1.Anonymous2.dmDisplayOrientation,
            )
        };
        if current_orientation != persisted_orientation {
            return Some(false);
        }
    }
    Some(true)
}
