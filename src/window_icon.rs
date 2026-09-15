//! Window icon acquisition, including the asynchronous WM_GETICON request
//! used by the original `WindowIcon` class.

use std::ptr::NonNull;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use windows_sys::Win32::Foundation::{HWND, LRESULT};
use windows_sys::Win32::Graphics::Gdi::{BITMAP, DeleteObject, GetObjectW, HBITMAP};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::UI::Shell::ExtractAssociatedIconW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CopyIcon, DestroyIcon, GCLP_HICON, GetClassLongPtrW, GetIconInfo, HICON, ICON_BIG, ICONINFO,
    IDI_APPLICATION, LoadIconW, SendMessageCallbackW, WM_GETICON,
};

type ChangedHandler = Rc<dyn Fn() + 'static>;

struct IconState {
    disposed: bool,
    icon: Option<OwnedIcon>,
    width: i32,
    height: i32,
    changed: Option<ChangedHandler>,
}

/// A private owner for icons returned by `CopyIcon` and
/// `ExtractAssociatedIconW`. Handles returned by `LoadIconW` and class-icon
/// lookups are deliberately never wrapped because they are shared.
struct OwnedIcon(NonNull<std::ffi::c_void>);

impl OwnedIcon {
    fn copy(icon: HICON) -> Result<Self, String> {
        let copied = unsafe { CopyIcon(icon) };
        Self::from_raw(copied).ok_or_else(|| "CopyIcon returned a null icon".to_string())
    }

    fn from_raw(icon: HICON) -> Option<Self> {
        NonNull::new(icon).map(Self)
    }

    fn raw(&self) -> HICON {
        self.0.as_ptr()
    }
}

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        unsafe {
            DestroyIcon(self.raw());
        }
    }
}

/// `GetIconInfo` transfers ownership of both returned bitmap handles.
struct IconInfoBitmaps {
    color: HBITMAP,
    mask: HBITMAP,
}

impl Drop for IconInfoBitmaps {
    fn drop(&mut self) {
        unsafe {
            if !self.color.is_null() {
                DeleteObject(self.color.cast());
            }
            if !self.mask.is_null() {
                DeleteObject(self.mask.cast());
            }
        }
    }
}

/// Owns a CopyIcon clone of the source window's icon.
pub struct WindowIcon {
    state: Rc<RefCell<IconState>>,
}

/// Non-owning icon state used by the asynchronous Changed callback.  A weak
/// handle prevents the callback from keeping a disposed `WindowIcon` alive.
pub struct WindowIconWeak {
    state: Weak<RefCell<IconState>>,
}

impl WindowIcon {
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is opaque, never dereferenced as Rust memory.
    pub fn new(source: HWND) -> Result<Self, String> {
        let class_icon = unsafe {
            GetClassLongPtrW(source, GCLP_HICON)
                as windows_sys::Win32::UI::WindowsAndMessaging::HICON
        };
        let extracted_icon = class_icon.is_null().then(program_icon).flatten();
        let source_icon = if !class_icon.is_null() {
            class_icon
        } else if let Some(icon) = extracted_icon.as_ref() {
            icon.raw()
        } else {
            unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) }
        };
        if source_icon.is_null() {
            return Err("LoadIconW returned a null fallback icon".to_string());
        }

        let owned_icon = OwnedIcon::copy(source_icon)?;
        let (width, height) = icon_size(owned_icon.raw())?;
        let state = Rc::new(RefCell::new(IconState {
            disposed: false,
            icon: Some(owned_icon),
            width,
            height,
            changed: None,
        }));

        // Keep the callback context alive until User32 calls the callback.
        // This is the same lifetime guarantee provided by the delegate field
        // in the original implementation. If User32 rejects the request, ownership
        // is immediately reclaimed below.
        // User32 invokes SendAsyncProc on this initiating UI thread: directly
        // for same-thread windows, or while this thread pumps messages for a
        // window on another thread. The retained Rc therefore never crosses
        // threads despite travelling through the untyped callback parameter.
        let context_ptr = Rc::into_raw(Rc::clone(&state));
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
                drop(Rc::from_raw(context_ptr));
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
        let mut state = self.state.borrow_mut();
        if !state.disposed {
            state.changed = Some(Rc::new(callback));
        }
    }

    pub fn weak(&self) -> WindowIconWeak {
        WindowIconWeak {
            state: Rc::downgrade(&self.state),
        }
    }

    /// Run a drawing operation while the cloned HICON remains alive.
    pub fn with_icon<R>(&self, f: impl FnOnce(HICON, i32, i32) -> R) -> Option<R> {
        let state = self.state.borrow();
        let icon = state.icon.as_ref()?;
        (!state.disposed).then(|| f(icon.raw(), state.width, state.height))
    }
}

impl WindowIconWeak {
    pub fn with_icon<R>(&self, f: impl FnOnce(HICON, i32, i32) -> R) -> Option<R> {
        let state_arc = self.state.upgrade()?;
        let state = state_arc.borrow();
        let icon = state.icon.as_ref()?;
        (!state.disposed).then(|| f(icon.raw(), state.width, state.height))
    }
}

impl Drop for WindowIcon {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        state.disposed = true;
        state.changed = None;
        state.icon = None;
    }
}

unsafe extern "system" fn icon_callback(_hwnd: HWND, _message: u32, data: usize, result: LRESULT) {
    // User32 transfers ownership of this context to the callback.
    let state_rc = unsafe { Rc::from_raw(data as *const RefCell<IconState>) };
    if result == 0 {
        return;
    }

    let icon = result as windows_sys::Win32::UI::WindowsAndMessaging::HICON;
    if icon.is_null() {
        return;
    }
    let owned_icon = match OwnedIcon::copy(icon) {
        Ok(icon) => icon,
        Err(_) => return,
    };
    let (width, height) = match icon_size(owned_icon.raw()) {
        Ok(size) => size,
        Err(_) => return,
    };

    let changed = {
        let mut state = state_rc.borrow_mut();
        if state.disposed {
            return;
        }
        state.icon = Some(owned_icon);
        state.width = width;
        state.height = height;
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

    let bitmaps = IconInfoBitmaps {
        color: info.hbmColor,
        mask: info.hbmMask,
    };
    let bitmap_handle = if !bitmaps.color.is_null() {
        bitmaps.color
    } else {
        bitmaps.mask
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
    if read == 0 {
        return Err("GetObjectW failed for the cloned window icon".to_string());
    }

    let height = if bitmaps.color.is_null() {
        bitmap.bmHeight / 2
    } else {
        bitmap.bmHeight
    };
    if bitmap.bmWidth <= 0 || height <= 0 {
        return Err("The cloned window icon has invalid dimensions".to_string());
    }
    Ok((bitmap.bmWidth, height))
}

fn program_icon() -> Option<OwnedIcon> {
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
    OwnedIcon::from_raw(icon)
}
