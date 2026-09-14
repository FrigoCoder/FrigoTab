//! Minimal owning wrappers around documented, broadly available Win32 APIs.
//!
//! The wrappers deliberately do not perform version checks.  They use APIs
//! available on the Windows versions supported by the original application
//! and report runtime failures (for example, DWM being unavailable) instead
//! of assuming a compositor or shell is present.

// Windows' opaque handle types are represented as raw pointers by
// `windows-sys`. Passing one to the documented FFI does not dereference it
// in Rust; the callee validates the handle. The public wrappers therefore
// remain safe while requiring callers to provide handles owned by Windows.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use core::ffi::c_void;
use core::mem::{size_of, zeroed};
use core::ptr::{null, null_mut};
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

use windows_sys::core::{BOOL, HRESULT};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, SetLastError, ERROR_ALREADY_EXISTS, ERROR_CLASS_ALREADY_EXISTS,
    ERROR_SUCCESS, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM,
};
use windows_sys::Win32::Graphics::Dwm::{
    DwmIsCompositionEnabled, DwmRegisterThumbnail, DwmUnregisterThumbnail,
    DwmUpdateThumbnailProperties, DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY,
    DWM_TNP_RECTDESTINATION, DWM_TNP_RECTSOURCE, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, GDI_ERROR, HBITMAP, HDC,
    HGDIOBJ, RGBQUAD, SRCCOPY,
};
use windows_sys::Win32::Storage::Xps::{PrintWindow, PW_CLIENTONLY};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{CreateMutexW, GetCurrentThreadId, ReleaseMutex};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetShellWindow, PostMessageW, PostQuitMessage, RegisterClassExW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, CS_HREDRAW, CS_VREDRAW, HOOKPROC, HWND_MESSAGE, MSG,
    WH_KEYBOARD_LL, WM_APP, WM_NULL, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_OVERLAPPED,
};

const CLASS_NAME: &str = "FrigoTab.Rust.Win32.Spike.MessageWindow";
const TRAY_CALLBACK_MESSAGE: u32 = WM_APP + 0x4f;

/// Errors returned by the small native wrappers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A Win32 function reported failure.  The code is the value returned by
    /// `GetLastError`.
    Win32 {
        /// Name of the failed operation.
        operation: &'static str,
        /// Win32 error code.
        code: u32,
    },
    /// A Win32 operation failed and its cleanup API returned a failure result.
    ///
    /// `cleanup_result` is the direct return value from the cleanup API; no
    /// `GetLastError` value is read because APIs such as `ReleaseDC` do not
    /// guarantee one.
    Win32WithCleanup {
        /// Name of the primary failed operation.
        operation: &'static str,
        /// Win32 error code from the primary operation.
        code: u32,
        /// Name of the cleanup operation.
        cleanup_operation: &'static str,
        /// Direct cleanup return value.
        cleanup_result: i32,
    },
    /// A Win32 API returned a failure result without a documented
    /// `GetLastError` contract.
    Win32Result {
        /// Name of the failed operation.
        operation: &'static str,
        /// Direct return value from the failed operation.
        result: i32,
    },
    /// An operation failed and its cleanup API returned a failure result,
    /// retaining the complete primary error when it is not a `Win32` error.
    WithCleanup {
        /// The primary operation error.
        primary: Box<Error>,
        /// Name of the cleanup operation.
        cleanup_operation: &'static str,
        /// Direct cleanup return value.
        cleanup_result: i32,
    },
    /// A COM-style API (including DWM) returned a failing HRESULT.
    HResult {
        /// Name of the failed operation.
        operation: &'static str,
        /// HRESULT as a signed 32-bit value.
        code: i32,
    },
    /// A native operation failed and its best-effort cleanup failed too.
    HResultWithCleanup {
        /// Name of the primary failed operation.
        operation: &'static str,
        /// HRESULT from the primary operation.
        code: i32,
        /// Name of the cleanup operation.
        cleanup_operation: &'static str,
        /// HRESULT from cleanup.
        cleanup_code: i32,
    },
    /// Another process already owns the requested named mutex.
    AlreadyRunning,
    /// The caller supplied a value that cannot be represented by the native
    /// API, such as a zero-sized capture surface.
    InvalidInput(&'static str),
}

/// Result type used by this crate.
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Win32 { operation, code } => {
                write!(f, "{operation} failed with Win32 error {code}")
            }
            Self::Win32WithCleanup {
                operation,
                code,
                cleanup_operation,
                cleanup_result,
            } => write!(
                f,
                "{operation} failed with Win32 error {code}; {cleanup_operation} cleanup returned {cleanup_result}"
            ),
            Self::Win32Result { operation, result } => {
                write!(f, "{operation} failed with native result {result}")
            }
            Self::WithCleanup {
                primary,
                cleanup_operation,
                cleanup_result,
            } => write!(
                f,
                "{primary}; {cleanup_operation} cleanup returned {cleanup_result}"
            ),
            Self::HResult { operation, code } => {
                write!(f, "{operation} failed with HRESULT 0x{code:08x}")
            }
            Self::HResultWithCleanup {
                operation,
                code,
                cleanup_operation,
                cleanup_code,
            } => write!(
                f,
                "{operation} failed with HRESULT 0x{code:08x}; {cleanup_operation} cleanup failed with HRESULT 0x{cleanup_code:08x}"
            ),
            Self::AlreadyRunning => f.write_str("another instance owns the named mutex"),
            Self::InvalidInput(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {}

fn win32_error(operation: &'static str) -> Error {
    Error::Win32 {
        operation,
        // SAFETY: GetLastError has no pointer or handle preconditions.
        code: unsafe { GetLastError() },
    }
}

fn check_bool(value: BOOL, operation: &'static str) -> Result<()> {
    if value == 0 {
        Err(win32_error(operation))
    } else {
        Ok(())
    }
}

fn check_hresult(value: HRESULT, operation: &'static str) -> Result<()> {
    if value < 0 {
        Err(Error::HResult {
            operation,
            code: value,
        })
    } else {
        Ok(())
    }
}

fn release_dc_checked(reference_window: HWND, screen_dc: HDC) -> std::result::Result<(), i32> {
    // SAFETY: `screen_dc` was returned by GetDC for `reference_window` and
    // this is the matching release operation. ReleaseDC reports failure via
    // its direct return value; it does not promise a meaningful last error.
    let result = unsafe { ReleaseDC(reference_window, screen_dc) };
    if result == 0 {
        Err(result)
    } else {
        Ok(())
    }
}

fn combine_release_dc_failure(primary: Error, cleanup_result: i32) -> Error {
    match primary {
        Error::Win32 { operation, code } => Error::Win32WithCleanup {
            operation,
            code,
            cleanup_operation: "ReleaseDC",
            cleanup_result,
        },
        primary => Error::WithCleanup {
            primary: Box::new(primary),
            cleanup_operation: "ReleaseDC",
            cleanup_result,
        },
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn copy_tip(destination: &mut [u16], value: &str) {
    destination.fill(0);
    let limit = destination.len().saturating_sub(1);
    for (slot, unit) in destination.iter_mut().take(limit).zip(value.encode_utf16()) {
        *slot = unit;
    }
}

fn register_window_class() -> Result<(Vec<u16>, HINSTANCE)> {
    let class_name = wide(CLASS_NAME);
    // SAFETY: A null module name asks Windows for the current process module.
    let module = unsafe { GetModuleHandleW(null()) } as HINSTANCE;
    if module.is_null() {
        return Err(win32_error("GetModuleHandleW"));
    }

    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(message_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: module,
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: null_mut(),
    };

    // SAFETY: `class` and its UTF-16 name remain alive for this synchronous call.
    let registered = unsafe { RegisterClassExW(&class) };
    if registered == 0 {
        // SAFETY: GetLastError has no pointer or handle preconditions.
        let error = unsafe { GetLastError() };
        if error != ERROR_CLASS_ALREADY_EXISTS {
            return Err(Error::Win32 {
                operation: "RegisterClassExW",
                code: error,
            });
        }
    }
    Ok((class_name, module))
}

/// A message-only window owned by the thread that created it.
///
/// Message-only windows never appear on the desktop and have no visual
/// surface.  The wrapper must be dropped on its creating thread, because
/// `DestroyWindow` is a thread-affine Win32 operation.  The window procedure
/// is intentionally bounded and delegates all unknown messages to
/// `DefWindowProcW`.
pub struct MessageOnlyWindow {
    hwnd: HWND,
    class_name: Vec<u16>,
}

impl MessageOnlyWindow {
    /// Create a message-only window using a stable class name.
    pub fn create() -> Result<Self> {
        let (class_name, module) = register_window_class()?;

        // SAFETY: The class/name pointers and null optional handles are valid
        // for this synchronous window creation call.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                null(),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                null_mut(),
                module,
                null(),
            )
        };
        if hwnd.is_null() {
            return Err(win32_error("CreateWindowExW"));
        }

        Ok(Self { hwnd, class_name })
    }

    /// Return the native window handle.
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Post an application message to this window.
    pub fn post(&self, message: u32, wparam: WPARAM, lparam: LPARAM) -> Result<()> {
        check_bool(
            // SAFETY: `self.hwnd` is owned by this wrapper and the message
            // parameters are copied by Windows before the call returns.
            unsafe { PostMessageW(self.hwnd, message, wparam, lparam) },
            "PostMessageW",
        )
    }

    /// Post a quit message to the current thread's queue.
    pub fn post_quit(exit_code: i32) {
        // SAFETY: PostQuitMessage only targets the current thread's queue.
        unsafe { PostQuitMessage(exit_code) }
    }
}

impl Drop for MessageOnlyWindow {
    fn drop(&mut self) {
        if !self.hwnd.is_null() {
            // The documented owner-thread requirement is part of this type's
            // contract; failure here cannot be usefully recovered during Drop.
            // SAFETY: Drop is required on the creating thread, and this HWND
            // remains owned by the wrapper until this call.
            unsafe {
                DestroyWindow(self.hwnd);
            }
        }
    }
}

// The class name is retained in the owner so the pointer passed to Win32 is
// always valid for the complete create call and to make the ownership intent
// explicit, even though Windows retains the registered class itself.
impl fmt::Debug for MessageOnlyWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageOnlyWindow")
            .field("hwnd", &self.hwnd)
            .field("class_name_units", &self.class_name.len())
            .finish()
    }
}

/// A hidden top-level owner window for opt-in native probes.
///
/// Unlike [`MessageOnlyWindow`], this HWND is top-level and can therefore be
/// used as a DWM thumbnail destination. It is never shown, activated, or
/// placed in the taskbar. Drop it on its creating thread.
pub struct HiddenOwnerWindow {
    hwnd: HWND,
    class_name: Vec<u16>,
}

impl HiddenOwnerWindow {
    /// Create a hidden, non-activating top-level owner window.
    pub fn create() -> Result<Self> {
        let (class_name, module) = register_window_class()?;
        // SAFETY: The class/name pointers and null optional handles are valid
        // for this synchronous window creation call.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                null(),
                WS_OVERLAPPED,
                0,
                0,
                1,
                1,
                null_mut(),
                null_mut(),
                module,
                null(),
            )
        };
        if hwnd.is_null() {
            return Err(win32_error("CreateWindowExW hidden owner"));
        }
        Ok(Self { hwnd, class_name })
    }

    /// Return the hidden top-level HWND.
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for HiddenOwnerWindow {
    fn drop(&mut self) {
        if !self.hwnd.is_null() {
            // SAFETY: Drop is required on the creating thread, and this HWND
            // remains owned by the wrapper until this call.
            unsafe {
                DestroyWindow(self.hwnd);
            }
        }
    }
}

impl fmt::Debug for HiddenOwnerWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HiddenOwnerWindow")
            .field("hwnd", &self.hwnd)
            .field("class_name_units", &self.class_name.len())
            .finish()
    }
}

/// Return the current shell desktop window, when Explorer (or another shell)
/// exposes one. A missing shell is a normal runtime condition for services,
/// alternate shells, and some remote sessions.
pub fn shell_window() -> Option<HWND> {
    // SAFETY: GetShellWindow has no pointer or handle arguments.
    let hwnd = unsafe { GetShellWindow() };
    (!hwnd.is_null()).then_some(hwnd)
}

unsafe extern "system" fn message_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // This callback has no Rust state and performs no allocation.  Keeping it
    // this small makes it suitable for a native message-loop smoke test.
    // SAFETY: Windows supplied the HWND/message parameters to this callback.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// The result of one `GetMessageW` iteration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessagePumpStep {
    /// A message was dispatched.
    Dispatched,
    /// Windows posted `WM_QUIT` and supplied this exit code.
    Quit(i32),
}

/// Pump one message from the current thread without blocking indefinitely in
/// this wrapper's own code.
pub fn pump_message() -> Result<MessagePumpStep> {
    let mut message = MSG::default();
    // SAFETY: `message` is a valid writable MSG and a null HWND filters the
    // current thread's queue.
    let status = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
    if status == -1 {
        return Err(win32_error("GetMessageW"));
    }
    if status == 0 {
        return Ok(MessagePumpStep::Quit(message.wParam as i32));
    }

    // SAFETY: `message` was filled by GetMessageW and remains alive here.
    unsafe {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    Ok(MessagePumpStep::Dispatched)
}

/// Run the current thread's message loop until `WM_QUIT`.
pub fn run_message_loop() -> Result<i32> {
    loop {
        match pump_message()? {
            MessagePumpStep::Dispatched => {}
            MessagePumpStep::Quit(code) => return Ok(code),
        }
    }
}

/// Return the current thread id, useful when assigning a hook or posting to a
/// dedicated message-loop thread.
pub fn current_thread_id() -> u32 {
    // SAFETY: GetCurrentThreadId has no pointer or handle arguments.
    unsafe { GetCurrentThreadId() }
}

/// A named mutex held for the lifetime of this value.
///
/// Drop this value on the thread that acquired it. `ReleaseMutex` is
/// ownership-aware and fails when called by a different thread. The non-`Send`
/// marker enforces that ownership rule; Drop performs best-effort teardown.
pub struct SingleInstance {
    handle: HANDLE,
    _thread_affine: PhantomData<Rc<()>>,
}

impl SingleInstance {
    /// Acquire a named, process-wide mutex.
    ///
    /// Names are passed as UTF-16 to the documented `CreateMutexW` API.  A
    /// named mutex is a kernel object and therefore works across processes;
    /// no version-specific shell or registration facility is involved.
    pub fn acquire(name: &str) -> Result<Self> {
        if name.is_empty() {
            return Err(Error::InvalidInput("mutex name must not be empty"));
        }
        let name = wide(name);
        // CreateMutexW only gives a meaningful last-error value for the
        // already-existing case. Clear it first so a stale error from an
        // unrelated native call cannot look like a second instance.
        // SAFETY: SetLastError only changes the calling thread's error slot.
        unsafe { SetLastError(ERROR_SUCCESS) };
        // SAFETY: The UTF-16 name is NUL-terminated and remains alive for the
        // synchronous CreateMutexW call; null attributes request defaults.
        let handle = unsafe { CreateMutexW(null(), 1, name.as_ptr()) };
        if handle.is_null() {
            return Err(win32_error("CreateMutexW"));
        }

        // SAFETY: GetLastError has no pointer or handle preconditions.
        let last_error = unsafe { GetLastError() };
        if last_error == ERROR_ALREADY_EXISTS {
            // SAFETY: `handle` is the newly returned kernel handle and is not
            // used after this close.
            unsafe {
                CloseHandle(handle);
            }
            return Err(Error::AlreadyRunning);
        }
        Ok(Self {
            handle,
            _thread_affine: PhantomData,
        })
    }

    /// Return the native mutex handle.
    pub fn handle(&self) -> HANDLE {
        self.handle
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: The mutex was acquired by this owner on this thread;
            // both calls consume only this wrapper's handle.
            unsafe {
                ReleaseMutex(self.handle);
                CloseHandle(self.handle);
            }
        }
    }
}

/// A shell notification-area icon owned by this value.
///
/// Keep it on the owner window's message-loop thread so callbacks and
/// deletion observe the same HWND lifetime.
pub struct TrayIcon {
    data: NOTIFYICONDATAW,
    added: bool,
    _thread_affine: PhantomData<Rc<()>>,
}

impl TrayIcon {
    /// Add an icon to the notification area.
    ///
    /// The callback is delivered to `window` as `callback_message`; callers
    /// should use a private `WM_APP` message and keep their window procedure
    /// bounded.  A null icon handle is accepted by the wrapper for API-surface
    /// probing but is normally rejected by the shell at runtime.
    pub fn add(
        window: HWND,
        id: u32,
        callback_message: u32,
        icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
        tooltip: &str,
    ) -> Result<Self> {
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: window,
            uID: id,
            uFlags: NIF_MESSAGE | NIF_ICON,
            uCallbackMessage: callback_message,
            hIcon: icon,
            // SAFETY: NOTIFYICONDATAW is a C ABI plain-data structure and a
            // zeroed union is the documented initial state.
            ..unsafe { zeroed() }
        };
        if !tooltip.is_empty() {
            data.uFlags |= NIF_TIP;
            copy_tip(&mut data.szTip, tooltip);
        }

        check_bool(
            // SAFETY: `data` is fully initialized and lives through the call.
            unsafe { Shell_NotifyIconW(NIM_ADD, &data) },
            "Shell_NotifyIconW(NIM_ADD)",
        )?;

        Ok(Self {
            data,
            added: true,
            _thread_affine: PhantomData,
        })
    }

    /// Change the tooltip without changing the icon registration.
    pub fn set_tooltip(&mut self, tooltip: &str) -> Result<()> {
        self.data.uFlags = NIF_TIP;
        copy_tip(&mut self.data.szTip, tooltip);
        check_bool(
            // SAFETY: `self.data` is owned and remains valid through the call.
            unsafe { Shell_NotifyIconW(NIM_MODIFY, &self.data) },
            "Shell_NotifyIconW(NIM_MODIFY)",
        )
    }

    /// Return the callback message configured for this icon.
    pub fn callback_message(&self) -> u32 {
        self.data.uCallbackMessage
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        if self.added {
            // SAFETY: The tray data remains owned by this value until Drop
            // completes, and the shell treats deletion as idempotent.
            unsafe {
                Shell_NotifyIconW(NIM_DELETE, &self.data);
            }
        }
    }
}

/// The lifetime of a global low-level keyboard hook registration.
pub struct LowLevelKeyboardHook {
    handle: windows_sys::Win32::UI::WindowsAndMessaging::HHOOK,
}

impl LowLevelKeyboardHook {
    /// Install a global `WH_KEYBOARD_LL` callback.
    ///
    /// # Safety
    ///
    /// The callback must be an `extern "system"` function that never panics
    /// or unwinds across the FFI boundary, performs only bounded work, and
    /// remains valid until this value is dropped.  The callback should call
    /// [`call_next_keyboard_hook`] for events it does not consume.  This
    /// function is intentionally not called by the crate's tests or smoke
    /// report.
    pub unsafe fn install(callback: HOOKPROC) -> Result<Self> {
        // SAFETY: A null module name asks Windows for the current process
        // module used by this static callback.
        let module = unsafe { GetModuleHandleW(null()) } as HINSTANCE;
        if module.is_null() {
            return Err(win32_error("GetModuleHandleW for keyboard hook"));
        }
        // SAFETY: The callback contract is documented on this unsafe method;
        // the module handle is valid for the process lifetime.
        let handle = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, callback, module, 0) };
        if handle.is_null() {
            return Err(win32_error("SetWindowsHookExW(WH_KEYBOARD_LL)"));
        }
        Ok(Self { handle })
    }

    /// Install an opt-in, pass-through hook for a short native smoke probe.
    ///
    /// The callback only forwards events and never consumes keys or allocates.
    /// The caller must keep pumping the installing thread's message queue and
    /// drop the returned value promptly. This helper is intentionally not
    /// used by tests or application startup.
    pub fn install_pass_through_probe() -> Result<Self> {
        // SAFETY: the callback is private, has a static lifetime, never
        // unwinds, and performs only the bounded CallNextHookEx operation.
        unsafe { Self::install(Some(pass_through_keyboard_hook_proc)) }
    }

    /// Return the native hook handle.
    pub fn handle(&self) -> windows_sys::Win32::UI::WindowsAndMessaging::HHOOK {
        self.handle
    }

    /// Uninstall the hook and return the native result to the caller.
    ///
    /// A failed close leaves the handle intact so [`Drop`] can make a second
    /// best-effort attempt. Calling this method more than once is harmless
    /// after the first successful close.
    pub fn close(&mut self) -> Result<()> {
        if self.handle.is_null() {
            return Ok(());
        }
        // SAFETY: `self.handle` was returned by SetWindowsHookExW and is
        // owned by this value until UnhookWindowsHookEx succeeds.
        if unsafe { UnhookWindowsHookEx(self.handle) } == 0 {
            return Err(win32_error("UnhookWindowsHookEx"));
        }
        self.handle = null_mut();
        Ok(())
    }

    /// Uninstall the hook and consume its owner.
    pub fn try_close(mut self) -> Result<()> {
        self.close()
    }
}

unsafe extern "system" fn pass_through_keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: Windows supplied these values to this callback; forwarding them
    // unchanged is the documented low-level hook contract.
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

impl Drop for LowLevelKeyboardHook {
    fn drop(&mut self) {
        // Drop remains best effort; explicit callers can use close/try_close
        // when they need the UnhookWindowsHookEx result.
        let _ = self.close();
    }
}

/// Forward an unconsumed low-level keyboard event to the next hook.
///
/// # Safety
///
/// `lparam` must be the value supplied by Windows to the hook callback.
pub unsafe fn call_next_keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: Windows supplied these values to this callback; forwarding them
    // unchanged is the documented low-level hook contract.
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

/// Report whether DWM composition is currently available.
///
/// DWM may be disabled or unavailable in a remote/session configuration.  A
/// caller should fall back to an opaque background in that case instead of
/// treating it as a version mismatch.
pub fn composition_enabled() -> Result<bool> {
    let mut enabled: BOOL = 0;
    check_hresult(
        // SAFETY: `enabled` is a valid writable BOOL for the synchronous call.
        unsafe { DwmIsCompositionEnabled(&mut enabled) },
        "DwmIsCompositionEnabled",
    )?;
    Ok(enabled != 0)
}

/// Properties used when updating a DWM thumbnail.
#[derive(Clone, Copy)]
pub struct ThumbnailUpdate {
    /// Destination rectangle in the owner window's client coordinates.
    pub destination: RECT,
    /// Whether DWM should make the thumbnail visible.
    pub visible: bool,
    /// Optional source rectangle in source-window coordinates.
    pub source: Option<RECT>,
    /// Optional opacity from 0 (transparent) to 255 (opaque).
    pub opacity: Option<u8>,
    /// Whether only the source client area should be included.
    pub source_client_area_only: Option<bool>,
}

impl ThumbnailUpdate {
    /// Create visible/hidden destination-only properties.
    pub const fn destination(destination: RECT, visible: bool) -> Self {
        Self {
            destination,
            visible,
            source: None,
            opacity: Some(255),
            source_client_area_only: None,
        }
    }
}

/// An owning DWM thumbnail registration.
///
/// Keep it on the destination window's owner thread; the non-`Send` marker
/// prevents accidental cross-thread teardown of the registration.
pub struct DwmThumbnail {
    handle: isize,
    _thread_affine: PhantomData<Rc<()>>,
}

impl DwmThumbnail {
    /// Register a source window's thumbnail in a destination window.
    pub fn register(destination: HWND, source: HWND) -> Result<Self> {
        let mut handle = 0isize;
        // SAFETY: `handle` is writable and the caller supplies opaque HWNDs
        // that DWM validates synchronously.
        let status = unsafe { DwmRegisterThumbnail(destination, source, &mut handle) };
        if status < 0 {
            if handle != 0 {
                // SAFETY: DWM returned this nonzero id during the failed
                // registration; unregistering it is the only safe cleanup.
                let cleanup_status = unsafe { DwmUnregisterThumbnail(handle) };
                if cleanup_status < 0 {
                    return Err(Error::HResultWithCleanup {
                        operation: "DwmRegisterThumbnail",
                        code: status,
                        cleanup_operation: "DwmUnregisterThumbnail",
                        cleanup_code: cleanup_status,
                    });
                }
            }
            return Err(Error::HResult {
                operation: "DwmRegisterThumbnail",
                code: status,
            });
        }
        if handle == 0 {
            return Err(Error::InvalidInput(
                "DwmRegisterThumbnail returned a null thumbnail",
            ));
        }
        Ok(Self {
            handle,
            _thread_affine: PhantomData,
        })
    }

    /// Update and explicitly set the thumbnail visibility bit.
    pub fn update(&self, update: ThumbnailUpdate) -> Result<()> {
        let mut properties = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_VISIBLE,
            rcDestination: update.destination,
            fVisible: if update.visible { 1 } else { 0 },
            // SAFETY: DWM_THUMBNAIL_PROPERTIES is a C ABI plain-data struct;
            // unspecified fields are documented as ignored when not flagged.
            ..unsafe { zeroed() }
        };

        if let Some(source) = update.source {
            properties.dwFlags |= DWM_TNP_RECTSOURCE;
            properties.rcSource = source;
        }
        if let Some(opacity) = update.opacity {
            properties.dwFlags |= DWM_TNP_OPACITY;
            properties.opacity = opacity;
        }
        if let Some(client_only) = update.source_client_area_only {
            properties.dwFlags |= DWM_TNP_SOURCECLIENTAREAONLY;
            properties.fSourceClientAreaOnly = if client_only { 1 } else { 0 };
        }

        check_hresult(
            // SAFETY: The registration owns `self.handle` and properties live
            // for this synchronous call.
            unsafe { DwmUpdateThumbnailProperties(self.handle, &properties) },
            "DwmUpdateThumbnailProperties",
        )
    }

    /// Return the opaque native thumbnail id.
    pub fn handle(&self) -> isize {
        self.handle
    }

    /// Unregister this thumbnail and return every DWM HRESULT to the caller.
    ///
    /// On success the registration is marked closed and Drop will not issue a
    /// second unregister call. On failure the handle remains available for a
    /// retry; Drop still performs a best-effort cleanup.
    pub fn close(&mut self) -> Result<()> {
        if self.handle == 0 {
            return Ok(());
        }
        check_hresult(
            // SAFETY: The registration owns this thumbnail id until the call
            // returns; Drop is best effort if DWM already tore it down.
            unsafe { DwmUnregisterThumbnail(self.handle) },
            "DwmUnregisterThumbnail",
        )?;
        self.handle = 0;
        Ok(())
    }

    /// Explicitly close this registration, consuming its owner.
    pub fn try_close(mut self) -> Result<()> {
        self.close()
    }
}

impl Drop for DwmThumbnail {
    fn drop(&mut self) {
        if self.handle != 0 {
            // DWM can already have torn down a source/destination window.  The
            // registration is still best-effort during Drop; update/register
            // paths report every HRESULT to callers.
            // SAFETY: The registration owns this thumbnail id until the call
            // returns; Drop is best effort if DWM already tore it down.
            unsafe {
                DwmUnregisterThumbnail(self.handle);
            }
        }
    }
}

/// An opt-in DWM probe that never shows a window or installs a production
/// mutex. It targets the shell desktop window in a hidden 1x1 owner and keeps
/// the thumbnail invisible and opaque, which makes it suitable for diagnosing
/// registration/update/cleanup support without disturbing the user.
pub struct ShellThumbnailProbe {
    thumbnail: DwmThumbnail,
    owner: HiddenOwnerWindow,
}

impl ShellThumbnailProbe {
    /// Create a hidden shell-thumbnail probe.
    ///
    /// This can fail when Explorer/another shell is absent or DWM is disabled;
    /// those are runtime capability results, not version checks. The helper
    /// does not call `ShowWindow`, activate the owner, or synthesize input.
    pub fn create() -> Result<Self> {
        let owner = HiddenOwnerWindow::create()?;
        let source = shell_window().ok_or(Error::InvalidInput(
            "the shell does not expose a desktop window",
        ))?;
        let thumbnail = DwmThumbnail::register(owner.hwnd(), source)?;
        thumbnail.update(ThumbnailUpdate::destination(
            RECT {
                left: 0,
                top: 0,
                right: 1,
                bottom: 1,
            },
            false,
        ))?;
        Ok(Self { thumbnail, owner })
    }

    /// Return the hidden owner HWND used by the probe.
    pub fn owner_hwnd(&self) -> HWND {
        self.owner.hwnd()
    }

    /// Explicitly unregister the thumbnail while retaining the hidden owner.
    pub fn close(&mut self) -> Result<()> {
        self.thumbnail.close()
    }

    /// Explicitly unregister the thumbnail and consume the probe.
    pub fn try_close(mut self) -> Result<()> {
        self.close()
    }
}

/// Opt-in rendering mode for [`GdiCapture::print_window_with_mode`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrintWindowMode {
    /// Use the documented baseline `PrintWindow` behavior.
    Standard,
    /// Try the widely implemented `PW_RENDERFULLCONTENT` value, then retry
    /// with [`PrintWindowMode::Standard`] if the target rejects it.
    BestEffortFullContent,
}

/// Result of a `PrintWindow` operation with an explicit mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrintWindowOutcome {
    /// The documented baseline flags succeeded.
    Standard,
    /// The opportunistic full-content flag succeeded.
    FullContent,
    /// Full-content was rejected, but the documented baseline retry succeeded.
    StandardFallback,
}

/// The widely implemented, but undocumented, `PrintWindow` full-content flag.
///
/// It is deliberately kept as an opt-in enhancement. The standard call is
/// always available as the compatibility fallback.
pub const PRINT_WINDOW_RENDER_FULL_CONTENT: u32 = 0x0002;

/// A 32-bit top-down DIB selected into a memory DC.
///
/// This is enough for a retained shell/window snapshot without introducing a
/// graphics framework.  The bitmap is restored before deletion and the DC is
/// released in `Drop`.  Capture methods require `&mut self` so a borrowed
/// pixel slice cannot coexist with a subsequent native write.
pub struct GdiCapture {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut c_void,
    width: usize,
    height: usize,
}

impl GdiCapture {
    /// Allocate a DIB-backed capture surface.
    pub fn new(reference_window: HWND, width: usize, height: usize) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(Error::InvalidInput("capture dimensions must be non-zero"));
        }
        let _bytes = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(Error::InvalidInput("capture dimensions overflow"))?;
        let width_i32 =
            i32::try_from(width).map_err(|_| Error::InvalidInput("capture width is too large"))?;
        let height_i32 = i32::try_from(height)
            .map_err(|_| Error::InvalidInput("capture height is too large"))?;

        // SAFETY: `reference_window` is an opaque HWND that GetDC validates;
        // null is the documented desktop DC request.
        let screen_dc = unsafe { GetDC(reference_window) };
        if screen_dc.is_null() {
            return Err(win32_error("GetDC"));
        }

        // SAFETY: `screen_dc` was returned non-null by GetDC and remains owned
        // until it is released below.
        let dc = unsafe { CreateCompatibleDC(screen_dc) };
        if dc.is_null() {
            // Capture the primary error before releasing the screen DC:
            // ReleaseDC has no documented GetLastError contract and may
            // overwrite the thread's last-error value.
            let primary = win32_error("CreateCompatibleDC");
            return match release_dc_checked(reference_window, screen_dc) {
                Ok(()) => Err(primary),
                Err(cleanup_result) => Err(combine_release_dc_failure(primary, cleanup_result)),
            };
        }

        let mut bits = null_mut();
        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width_i32,
                biHeight: -height_i32,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            bmiColors: [RGBQUAD::default()],
        };
        // SAFETY: `bitmap_info` and `bits` are valid for this synchronous call;
        // the screen DC remains acquired until the call returns.
        let bitmap = unsafe {
            CreateDIBSection(
                screen_dc,
                &bitmap_info,
                DIB_RGB_COLORS,
                &mut bits,
                null_mut(),
                0,
            )
        };

        // Capture a CreateDIBSection failure before ReleaseDC can change the
        // thread's last-error value. A non-null bitmap with null bits is an
        // invalid native result, not a reason to read a stale last error.
        let allocation_error = if bitmap.is_null() {
            Some(win32_error("CreateDIBSection"))
        } else if bits.is_null() {
            Some(Error::InvalidInput(
                "CreateDIBSection returned a null bits pointer",
            ))
        } else {
            None
        };
        let release_result = release_dc_checked(reference_window, screen_dc);

        if let Some(primary) = allocation_error {
            // SAFETY: The bitmap has not been selected into `dc`; both
            // handles were created by this constructor and remain owned here.
            // ReleaseDC was attempted above even when it reported failure.
            unsafe {
                if !bitmap.is_null() {
                    DeleteObject(bitmap as HGDIOBJ);
                }
                DeleteDC(dc);
            }
            return match release_result {
                Ok(()) => Err(primary),
                Err(cleanup_result) => Err(combine_release_dc_failure(primary, cleanup_result)),
            };
        }

        if let Err(cleanup_result) = release_result {
            // SAFETY: The bitmap has not been selected into `dc`; both
            // handles were created by this constructor and remain owned here.
            // They must still be destroyed when ReleaseDC reports failure.
            unsafe {
                DeleteObject(bitmap as HGDIOBJ);
                DeleteDC(dc);
            }
            return Err(Error::Win32Result {
                operation: "ReleaseDC",
                result: cleanup_result,
            });
        }

        // SAFETY: `dc` is a newly created memory DC and `bitmap` is a newly
        // allocated compatible DIB, both valid for this call.
        let previous = unsafe { SelectObject(dc, bitmap as HGDIOBJ) };
        if previous.is_null() || previous == (GDI_ERROR as isize as HGDIOBJ) {
            // SAFETY: SelectObject failed, so the bitmap is not selected into
            // the DC; both handles remain owned by this constructor.
            unsafe {
                DeleteObject(bitmap as HGDIOBJ);
                DeleteDC(dc);
            }
            return Err(win32_error("SelectObject"));
        }

        Ok(Self {
            dc,
            bitmap,
            previous,
            bits,
            width,
            height,
        })
    }

    /// Copy pixels from another DC using the documented `SRCCOPY` raster op.
    pub fn bit_blt_from(&mut self, source: HDC) -> Result<()> {
        if source.is_null() {
            return Err(Error::InvalidInput("source DC must not be null"));
        }
        check_bool(
            // SAFETY: `self.dc` and source are opaque handles validated by
            // BitBlt; dimensions were checked during construction.
            unsafe {
                BitBlt(
                    self.dc,
                    0,
                    0,
                    self.width as i32,
                    self.height as i32,
                    source,
                    0,
                    0,
                    SRCCOPY,
                )
            },
            "BitBlt",
        )
    }

    /// Ask a window to render into the retained bitmap.
    ///
    /// Some accelerated or protected windows may refuse `PrintWindow`; that
    /// is a runtime capture limitation and is returned to the caller so it
    /// can use a fallback.
    pub fn print_window(&mut self, source: HWND, client_only: bool) -> Result<()> {
        self.print_window_with_mode(source, client_only, PrintWindowMode::Standard)
            .map(|_| ())
    }

    /// Render a window with an explicit baseline or best-effort full-content
    /// mode. The full-content value is not part of the documented contract;
    /// failures are handled by retrying the standard call before returning an
    /// error.
    pub fn print_window_with_mode(
        &mut self,
        source: HWND,
        client_only: bool,
        mode: PrintWindowMode,
    ) -> Result<PrintWindowOutcome> {
        let standard_flags = if client_only { PW_CLIENTONLY } else { 0 };
        match mode {
            PrintWindowMode::Standard => {
                check_bool(
                    // SAFETY: `self.dc` is the owned capture surface and
                    // `source` is an opaque HWND validated by PrintWindow.
                    unsafe { PrintWindow(source, self.dc, standard_flags) },
                    "PrintWindow",
                )?;
                Ok(PrintWindowOutcome::Standard)
            }
            PrintWindowMode::BestEffortFullContent => {
                if {
                    // SAFETY: `self.dc` is the owned capture surface and
                    // `source` is an opaque HWND validated by PrintWindow.
                    unsafe {
                        PrintWindow(
                            source,
                            self.dc,
                            standard_flags | PRINT_WINDOW_RENDER_FULL_CONTENT,
                        )
                    }
                } != 0
                {
                    return Ok(PrintWindowOutcome::FullContent);
                }
                check_bool(
                    // SAFETY: `self.dc` is the owned capture surface and
                    // `source` is an opaque HWND validated by PrintWindow.
                    unsafe { PrintWindow(source, self.dc, standard_flags) },
                    "PrintWindow (standard fallback)",
                )?;
                Ok(PrintWindowOutcome::StandardFallback)
            }
        }
    }

    /// Return the DIB's BGRA bytes in top-down row order.
    pub fn pixels(&self) -> &[u8] {
        let length = self.width * self.height * 4;
        // SAFETY: `bits` is returned by CreateDIBSection for `bitmap`, remains
        // valid until Drop, and the slice length was checked for overflow in
        // new().  The borrow prevents the caller from mutating through this
        // wrapper while the slice is live.
        // SAFETY: CreateDIBSection returned this pointer for the owned bitmap;
        // its checked byte length remains valid until Drop.
        unsafe { core::slice::from_raw_parts(self.bits.cast::<u8>(), length) }
    }

    /// Return the memory DC.
    pub fn dc(&self) -> HDC {
        self.dc
    }

    /// Return the dimensions in pixels.
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }
}

impl Drop for GdiCapture {
    fn drop(&mut self) {
        // SAFETY: The selected bitmap is restored before deleting the owned
        // bitmap and memory DC.
        unsafe {
            if !self.dc.is_null() && !self.previous.is_null() {
                SelectObject(self.dc, self.previous);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap as HGDIOBJ);
            }
            if !self.dc.is_null() {
                DeleteDC(self.dc);
            }
        }
    }
}

/// A private message value reserved for future tray/session experiments.
pub const PRIVATE_CALLBACK_MESSAGE: u32 = TRAY_CALLBACK_MESSAGE;

/// A harmless compile-time reference to the native message constants used by
/// the spike.  It exists to make API coverage visible to contract tests.
pub const NATIVE_MESSAGE_SENTINEL: u32 = WM_NULL;

#[cfg(test)]
#[path = "t20260914t181600z_win32_contract.rs"]
mod t20260914t181600z_win32_contract;
