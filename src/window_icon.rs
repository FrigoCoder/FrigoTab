//! Window icon acquisition, including the asynchronous WM_GETICON request
//! used by the original `WindowIcon` class.

use std::sync::{Arc, Mutex, Weak};

use windows_sys::Win32::Foundation::{HWND, LRESULT};
use windows_sys::Win32::Graphics::Gdi::{BITMAP, DeleteObject, GetObjectW};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::UI::Shell::ExtractAssociatedIconW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CopyIcon, DestroyIcon, GCLP_HICON, GetClassLongPtrW, GetIconInfo, HICON, ICON_BIG, ICONINFO,
    IDI_APPLICATION, LoadIconW, SendMessageCallbackW, WM_GETICON,
};

type ChangedHandler = Arc<dyn Fn() + 'static>;

struct IconState {
    disposed: bool,
    hicon: HICON,
    width: i32,
    height: i32,
    changed: Option<ChangedHandler>,
}

struct CallbackContext {
    state: Arc<Mutex<IconState>>,
}

/// Owns a CopyIcon clone of the source window's icon.
pub struct WindowIcon {
    state: Arc<Mutex<IconState>>,
}

/// Non-owning icon state used by the asynchronous Changed callback.  A weak
/// handle prevents the callback from keeping a disposed `WindowIcon` alive.
pub struct WindowIconWeak {
    state: Weak<Mutex<IconState>>,
}

impl WindowIcon {
    pub fn new(source: HWND) -> Result<Self, String> {
        let class_icon = unsafe {
            GetClassLongPtrW(source, GCLP_HICON)
                as windows_sys::Win32::UI::WindowsAndMessaging::HICON
        };
        let mut destroy_source = false;
        let source_icon = if class_icon.is_null() {
            if let Some(icon) = program_icon() {
                destroy_source = true;
                icon
            } else {
                unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) }
            }
        } else {
            class_icon
        };
        if source_icon.is_null() {
            return Err("LoadIconW returned a null fallback icon".to_string());
        }

        let owned_icon = unsafe { CopyIcon(source_icon) };
        if destroy_source {
            unsafe {
                DestroyIcon(source_icon);
            }
        }
        if owned_icon.is_null() {
            return Err("CopyIcon returned a null icon".to_string());
        }
        let (width, height) = match icon_size(owned_icon) {
            Ok(size) => size,
            Err(error) => {
                unsafe {
                    DestroyIcon(owned_icon);
                }
                return Err(error);
            }
        };
        let state = Arc::new(Mutex::new(IconState {
            disposed: false,
            hicon: owned_icon,
            width,
            height,
            changed: None,
        }));

        // Keep the callback context alive until User32 calls the callback.
        // This is the same lifetime guarantee provided by the delegate field
        // in the original implementation. If User32 rejects the request, ownership
        // is immediately reclaimed below.
        let context = Box::new(CallbackContext {
            state: Arc::clone(&state),
        });
        let context_ptr = Box::into_raw(context);
        let queued = unsafe {
            SendMessageCallbackW(
                source,
                WM_GETICON,
                ICON_BIG as usize,
                0,
                Some(icon_callback),
                context_ptr as usize,
            )
        };
        if queued == 0 {
            unsafe {
                drop(Box::from_raw(context_ptr));
            }
        }

        Ok(Self { state })
    }

    /// Install the equivalent of the original `Changed` event. The callback is
    /// invoked after a valid asynchronous WM_GETICON replacement is cloned.
    pub fn on_changed<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !state.disposed {
            state.changed = Some(Arc::new(callback));
        }
    }

    pub fn weak(&self) -> WindowIconWeak {
        WindowIconWeak {
            state: Arc::downgrade(&self.state),
        }
    }

    /// Run a drawing operation while the cloned HICON remains alive.
    pub fn with_icon<R>(&self, f: impl FnOnce(HICON, i32, i32) -> R) -> Option<R> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.disposed {
            return None;
        }
        Some(f(state.hicon, state.width, state.height))
    }
}

impl WindowIconWeak {
    pub fn with_icon<R>(&self, f: impl FnOnce(HICON, i32, i32) -> R) -> Option<R> {
        let state_arc = self.state.upgrade()?;
        let state = state_arc
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.disposed {
            return None;
        }
        Some(f(state.hicon, state.width, state.height))
    }
}

impl Drop for WindowIcon {
    fn drop(&mut self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.disposed = true;
        state.changed = None;
        let icon = state.hicon;
        state.hicon = std::ptr::null_mut();
        if !icon.is_null() {
            unsafe {
                DestroyIcon(icon);
            }
        }
    }
}

unsafe extern "system" fn icon_callback(_hwnd: HWND, _message: u32, data: usize, result: LRESULT) {
    // User32 transfers ownership of this context to the callback.
    let context = unsafe { Box::from_raw(data as *mut CallbackContext) };
    if result == 0 {
        return;
    }

    let icon = result as windows_sys::Win32::UI::WindowsAndMessaging::HICON;
    if icon.is_null() {
        return;
    }
    let owned_icon = unsafe { CopyIcon(icon) };
    if owned_icon.is_null() {
        return;
    }
    let (width, height) = match icon_size(owned_icon) {
        Ok(size) => size,
        Err(_) => {
            unsafe {
                DestroyIcon(owned_icon);
            }
            return;
        }
    };

    let changed = {
        let mut state = context
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.disposed {
            unsafe {
                DestroyIcon(owned_icon);
            }
            return;
        }
        let previous_icon = state.hicon;
        state.hicon = owned_icon;
        state.width = width;
        state.height = height;
        unsafe {
            DestroyIcon(previous_icon);
        }
        state.changed.clone()
    };
    if let Some(callback) = changed {
        callback();
    }
}

fn icon_size(icon: HICON) -> Result<(i32, i32), String> {
    let mut info = ICONINFO::default();
    if unsafe { GetIconInfo(icon, &mut info) } == 0 {
        return Err("GetIconInfo failed for the cloned window icon".to_string());
    }

    let bitmap_handle = if !info.hbmColor.is_null() {
        info.hbmColor
    } else {
        info.hbmMask
    };
    let mut bitmap = BITMAP::default();
    let read = if bitmap_handle.is_null() {
        0
    } else {
        unsafe {
            GetObjectW(
                bitmap_handle.cast(),
                std::mem::size_of::<BITMAP>() as i32,
                (&mut bitmap as *mut BITMAP).cast(),
            )
        }
    };
    unsafe {
        if !info.hbmColor.is_null() {
            DeleteObject(info.hbmColor.cast());
        }
        if !info.hbmMask.is_null() {
            DeleteObject(info.hbmMask.cast());
        }
    }
    if read == 0 {
        return Err("GetObjectW failed for the cloned window icon".to_string());
    }

    let height = if info.hbmColor.is_null() {
        bitmap.bmHeight / 2
    } else {
        bitmap.bmHeight
    };
    if bitmap.bmWidth <= 0 || height <= 0 {
        return Err("The cloned window icon has invalid dimensions".to_string());
    }
    Ok((bitmap.bmWidth, height))
}

fn program_icon() -> Option<HICON> {
    let mut path = vec![0u16; 32_768];
    let length =
        unsafe { GetModuleFileNameW(std::ptr::null_mut(), path.as_mut_ptr(), path.len() as u32) };
    if length == 0 || length >= path.len() as u32 {
        return None;
    }
    path[length as usize] = 0;
    let mut icon_index = 0u16;
    let icon =
        unsafe { ExtractAssociatedIconW(std::ptr::null_mut(), path.as_mut_ptr(), &mut icon_index) };
    (!icon.is_null()).then_some(icon)
}
