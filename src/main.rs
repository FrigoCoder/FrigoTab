#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::mem::{size_of, transmute};
use std::ptr::{null, null_mut};

use frigotab::key_handling::KeyHandling;
use frigotab::key_hook::{KeyHook, WM_KEY_HOOK_INPUT};
use frigotab::keyboard_input::{KeyTransition, KeyboardInput, SwitcherKey};
use frigotab::rect::virtual_screen_bounds;
use frigotab::screen_point::ScreenPoint;
use frigotab::session_window::{SessionPainter, SessionWindow, WM_DESKTOP_SNAPSHOT_READY};
use frigotab::single_instance_guard::{APPLICATION_MUTEX_NAME, SingleInstanceGuard};
use frigotab::switcher_application::SwitcherApplication;
use frigotab::switcher_state::SwitcherState;
use frigotab::sys_tray_icon::{SysTrayIcon, TRAY_CALLBACK_MESSAGE, TrayAction};
use frigotab::window_handle::WindowHandle;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{BeginPaint, ClientToScreen, EndPaint, PAINTSTRUCT};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, IDC_ARROW, LoadCursorW,
    MA_ACTIVATE, MB_ICONERROR, MB_OK, MSG, MessageBoxW, PostMessageW, PostQuitMessage,
    RegisterClassExW, SetProcessDPIAware, SetWindowLongPtrW, TranslateMessage, WM_ACTIVATEAPP,
    WM_CLOSE, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_DWMCOMPOSITIONCHANGED, WM_ENDSESSION,
    WM_ERASEBKGND, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCCREATE, WM_NCDESTROY,
    WM_PAINT, WM_QUERYENDSESSION, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const OWNER_CLASS: &str = "FrigoTab.SessionOwner";
const OWNER_TITLE: &str = "FrigoTab";
const WM_BEGIN_SESSION: u32 = 0x4001;
#[cfg(debug_assertions)]
const DEBUG_TIMER_ID: usize = 1;

/// The one UI-thread-owned object graph corresponding to `Program.Run`.
/// `SessionWindow` is kept separate from the deterministic controller just as
/// `SessionWindow` is separate from `SwitcherApplication` in the original
/// application.
struct App {
    controller: SwitcherApplication,
    session: Option<SessionWindow>,
    hook: Option<KeyHook>,
    tray: Option<SysTrayIcon>,
    reported_session_visibility: bool,
}

/// Stable userdata for the owner HWND. The painter deliberately lives beside,
/// rather than inside, the dynamically borrowed application state so a
/// synchronous WM_PAINT can safely repaint during native session operations.
struct OwnerContext {
    app: RefCell<App>,
    painter: Option<SessionPainter>,
}

impl App {
    fn new() -> Self {
        Self {
            controller: SwitcherApplication::new(),
            session: None,
            hook: None,
            tray: None,
            reported_session_visibility: false,
        }
    }

    fn handle_keyboard(&mut self, input: KeyboardInput) -> KeyHandling {
        let handling = if let Some(session) = self.session.as_mut() {
            self.controller.handle_keyboard(session, input)
        } else {
            KeyHandling::PassThrough
        };
        self.sync_session_visibility();
        handling
    }

    fn handle_mouse_move(&mut self, point: ScreenPoint) {
        if let Some(session) = self.session.as_mut() {
            self.controller.handle_mouse_move(session, point);
            self.sync_session_visibility();
        }
    }

    fn handle_mouse_click(&mut self, point: ScreenPoint) {
        if let Some(session) = self.session.as_mut() {
            self.controller.handle_mouse_click(session, point);
            self.sync_session_visibility();
        }
    }

    fn begin_session(&mut self) {
        let input = KeyboardInput::new(SwitcherKey::Tab, KeyTransition::Down, true, false, false);
        let _ = self.handle_keyboard(input);
    }

    fn dispatch_hook_message(&mut self, wparam: WPARAM) {
        let Some(hook) = self.hook.as_ref() else {
            return;
        };
        // `dispatch_ui_message` invokes the handler synchronously on this UI
        // thread. A raw pointer avoids borrowing `self.hook` across the
        // closure while keeping the KeyHook alive for the entire call.
        let hook = hook as *const KeyHook;
        unsafe {
            (*hook).dispatch_ui_message(wparam, |input| self.handle_keyboard(input));
        }
    }

    fn sync_session_visibility(&mut self) {
        let visible = self.controller.state() == SwitcherState::Visible;
        if visible != self.reported_session_visibility {
            if let Some(hook) = self.hook.as_ref() {
                hook.set_session_visible(visible);
            }
            self.reported_session_visibility = visible;
        }
    }

    fn interrupt(&mut self) {
        if let Some(session) = self.session.as_mut() {
            self.controller.interrupt(session);
        }
        if let Some(hook) = self.hook.as_ref() {
            hook.reset_input_state();
        }
        self.sync_session_visibility();
    }

    fn close_for_shutdown(&mut self) {
        if let Some(session) = self.session.as_mut() {
            // SessionForm.Dispose marks itself disposed before controller.Close,
            // which prevents CloseSessionResources from queuing a final shell
            // capture while the application is shutting down.
            session.dispose();
            self.controller.close(session);
        }
        self.sync_session_visibility();
        if let Some(hook) = self.hook.take() {
            hook.drain_ui_messages();
            drop(hook);
        }
        // Remove the notification-area registration while the owner HWND is
        // still valid.  This prevents a failed NIM_DELETE from leaving a
        // stale icon after the owner is destroyed.
        drop(self.tray.take());
    }

    fn relayout(&mut self) {
        let was_visible = self.controller.state() == SwitcherState::Visible;
        if let Some(session) = self.session.as_mut() {
            self.controller.relayout(session);
            if was_visible
                && self.controller.state() != SwitcherState::Visible
                && let Some(hook) = self.hook.as_ref()
            {
                hook.reset_input_state();
            }
            session.queue_desktop_snapshot_refresh_current();
        }
        self.sync_session_visibility();
    }

    fn publish_snapshot(&mut self) {
        if let Some(session) = self.session.as_mut() {
            session.publish_desktop_snapshot();
        }
    }
}

fn main() {
    enable_dpi_awareness();
    let Some(_instance) = acquire_instance() else {
        return;
    };

    let instance = unsafe { GetModuleHandleW(null()) };
    if instance.is_null() {
        return;
    }
    let class_name = wide(OWNER_CLASS);
    let title = wide(OWNER_TITLE);
    if register_owner_class(instance, &class_name) == 0 {
        return;
    }

    let bounds = virtual_screen_bounds();
    if bounds.right <= bounds.left || bounds.bottom <= bounds.top {
        return;
    }

    // Win32 calls can synchronously re-enter this WndProc. RefCell turns
    // those native callback boundaries into checked Rust borrows instead of
    // manufacturing aliased `&mut App` references from the userdata pointer.
    let mut context = Box::new(OwnerContext {
        app: RefCell::new(App::new()),
        painter: None,
    });
    let context_pointer = (&mut *context) as *mut OwnerContext;
    let owner = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            bounds.left,
            bounds.top,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            null_mut(),
            null_mut(),
            instance,
            context_pointer.cast(),
        )
    };
    if owner.is_null() {
        return;
    }

    // SessionWindow captures Explorer while this owner remains hidden. This
    // is the same startup ordering as SessionForm's retained desktop frame.
    let session = SessionWindow::new(owner);
    context.painter = Some(session.painter());
    context.app.get_mut().session = Some(session);
    let app = &context.app;
    let tray = match SysTrayIcon::new(owner) {
        Ok(tray) => tray,
        Err(error) => {
            show_error(&format!(
                "FrigoTab could not create its tray icon (Win32 error {error})."
            ));
            {
                let mut app = app.borrow_mut();
                app.close_for_shutdown();
                if let Some(session) = app.session.as_mut() {
                    session.dispose();
                }
            }
            unsafe {
                DestroyWindow(owner);
            }
            return;
        }
    };
    app.borrow_mut().tray = Some(tray);
    let hook = match KeyHook::start(owner) {
        Ok(hook) => hook,
        Err(error) => {
            show_error(&format!(
                "FrigoTab could not install its global keyboard hook.\n\n{error}"
            ));
            {
                let mut app = app.borrow_mut();
                app.close_for_shutdown();
                if let Some(session) = app.session.as_mut() {
                    session.dispose();
                }
            }
            unsafe {
                DestroyWindow(owner);
            }
            return;
        }
    };
    app.borrow_mut().hook = Some(hook);

    #[cfg(debug_assertions)]
    unsafe {
        let _ = windows_sys::Win32::UI::WindowsAndMessaging::SetTimer(
            owner,
            DEBUG_TIMER_ID,
            10_000,
            None,
        );
    }

    let mut message = MSG::default();
    loop {
        let result = unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::GetMessageW(&mut message, null_mut(), 0, 0)
        };
        if result <= 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    {
        let mut app = app.borrow_mut();
        app.close_for_shutdown();
        if let Some(session) = app.session.as_mut() {
            session.dispose();
        }
    }
    if WindowHandle::new(owner).is_valid() {
        unsafe {
            DestroyWindow(owner);
        }
    }
    drop(context);
}

fn acquire_instance() -> Option<SingleInstanceGuard> {
    match SingleInstanceGuard::try_acquire(APPLICATION_MUTEX_NAME) {
        Ok(Some(guard)) => Some(guard),
        Ok(None) => None,
        Err(error) => {
            show_error(&format!(
                "FrigoTab could not acquire its application mutex.\n\n{error}"
            ));
            None
        }
    }
}

fn register_owner_class(
    instance: windows_sys::Win32::Foundation::HMODULE,
    class_name: &[u16],
) -> u16 {
    let cursor = unsafe { LoadCursorW(null_mut(), IDC_ARROW) };
    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(owner_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: null_mut(),
        hCursor: cursor,
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: null_mut(),
    };
    unsafe { RegisterClassExW(&class) }
}

unsafe extern "system" fn owner_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        if !create.is_null() {
            let context = unsafe { (*create).lpCreateParams as *mut OwnerContext };
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, context as isize);
            }
            return 1;
        }
    }

    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OwnerContext };
    if pointer.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    // The Box holding this context outlives its owner HWND. Native calls may
    // synchronously enter this function again, so application branches use a
    // checked, tightly-scoped dynamic borrow instead of an `*mut App`.
    let context = unsafe { &*pointer };
    let app = &context.app;

    // TrackPopupMenu pumps a nested message loop. Keep no Ref/RefMut alive
    // across it so a reentrant WM_CLOSE can perform the normal teardown.
    if message == TRAY_CALLBACK_MESSAGE {
        let settings = {
            let Ok(app) = app.try_borrow() else {
                return 0;
            };
            app.tray.as_ref().map(|_| {
                (
                    app.controller.alt_tab_behavior(),
                    app.session
                        .as_ref()
                        .map(SessionWindow::background_mode)
                        .unwrap_or_default(),
                )
            })
        };
        let Some((alt_tab_behavior, background_mode)) = settings else {
            return 0;
        };
        let action = SysTrayIcon::handle_callback(hwnd, lparam, alt_tab_behavior, background_mode);
        if WindowHandle::new(hwnd).is_valid()
            && unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OwnerContext == pointer }
        {
            let mut destroy = false;
            if let Ok(mut app) = app.try_borrow_mut() {
                match action {
                    Some(TrayAction::SetAltTabBehavior(behavior)) => {
                        app.controller.set_alt_tab_behavior(behavior);
                    }
                    Some(TrayAction::SetBackgroundMode(mode)) => {
                        if let Some(session) = app.session.as_mut() {
                            session.set_background_mode(mode);
                        }
                    }
                    Some(TrayAction::Exit) => {
                        app.close_for_shutdown();
                        destroy = true;
                    }
                    None => {}
                }
            }
            // DestroyWindow synchronously sends WM_DESTROY/WM_NCDESTROY.
            // Wait until the RefMut above has ended before entering it.
            if destroy {
                unsafe {
                    DestroyWindow(hwnd);
                }
            }
        }
        return 0;
    }

    match message {
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = unsafe { BeginPaint(hwnd, &mut paint) };
            let mut client = RECT::default();
            unsafe {
                GetClientRect(hwnd, &mut client);
            }
            if let Some(painter) = context.painter.as_ref() {
                painter.paint(dc, client);
            }
            unsafe {
                EndPaint(hwnd, &paint);
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => MA_ACTIVATE as isize,
        WM_MOUSEMOVE => {
            if let Ok(mut app) = app.try_borrow_mut() {
                app.handle_mouse_move(mouse_screen_point(hwnd, lparam));
            }
            0
        }
        WM_LBUTTONDOWN => {
            if let Ok(mut app) = app.try_borrow_mut() {
                app.handle_mouse_click(mouse_screen_point(hwnd, lparam));
            }
            0
        }
        WM_ACTIVATEAPP => {
            if wparam == 0
                && let Ok(mut app) = app.try_borrow_mut()
            {
                let activating = app
                    .session
                    .as_ref()
                    .is_some_and(SessionWindow::activating_selection);
                if app.controller.state() == SwitcherState::Visible && !activating {
                    app.interrupt();
                }
            }
            0
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED | WM_DWMCOMPOSITIONCHANGED => {
            if let Ok(mut app) = app.try_borrow_mut() {
                app.relayout();
            }
            0
        }
        WM_ENDSESSION => {
            if let Ok(mut app) = app.try_borrow_mut() {
                app.interrupt();
            }
            0
        }
        WM_QUERYENDSESSION => {
            if let Ok(mut app) = app.try_borrow_mut() {
                app.interrupt();
            }
            1
        }
        WM_BEGIN_SESSION => {
            if let Ok(mut app) = app.try_borrow_mut() {
                app.begin_session();
            }
            0
        }
        WM_DESKTOP_SNAPSHOT_READY => {
            if let Ok(mut app) = app.try_borrow_mut() {
                app.publish_snapshot();
            }
            0
        }
        WM_KEY_HOOK_INPUT => {
            if let Ok(mut app) = app.try_borrow_mut() {
                app.dispatch_hook_message(wparam);
            }
            0
        }
        #[cfg(debug_assertions)]
        windows_sys::Win32::UI::WindowsAndMessaging::WM_TIMER if wparam == DEBUG_TIMER_ID => {
            let can_destroy = if let Ok(mut app) = app.try_borrow_mut() {
                app.close_for_shutdown();
                true
            } else {
                false
            };
            if can_destroy {
                unsafe {
                    DestroyWindow(hwnd);
                }
            }
            0
        }
        WM_CLOSE => {
            let can_destroy = if let Ok(mut app) = app.try_borrow_mut() {
                app.close_for_shutdown();
                true
            } else {
                false
            };
            if can_destroy {
                unsafe {
                    DestroyWindow(hwnd);
                }
            } else {
                // Retry after the native call which caused this reentrancy
                // returns and releases its checked application borrow.
                unsafe {
                    PostMessageW(hwnd, WM_CLOSE, 0, 0);
                }
            }
            0
        }
        WM_DESTROY => {
            // WM_DESTROY is also reachable when the owner is destroyed by
            // another path than WM_CLOSE. Finish the worker/resource teardown
            // while this HWND cannot yet be reused by another window.
            if let Ok(mut app) = app.try_borrow_mut() {
                app.close_for_shutdown();
            }
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        WM_NCDESTROY => {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn enable_dpi_awareness() {
    let user32 = wide("user32.dll");
    let module = unsafe { GetModuleHandleW(user32.as_ptr()) };
    if !module.is_null() {
        let name = b"SetProcessDpiAwarenessContext\0";
        if let Some(address) = unsafe { GetProcAddress(module, name.as_ptr()) } {
            let set_context: unsafe extern "system" fn(isize) -> i32 =
                unsafe { transmute(address) };
            if unsafe { set_context(-4) } != 0 {
                return;
            }
        }
    }
    unsafe {
        let _ = SetProcessDPIAware();
    }
}

fn show_error(message: &str) {
    let text = wide(message);
    let title = wide(OWNER_TITLE);
    unsafe {
        MessageBoxW(
            null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn mouse_screen_point(hwnd: HWND, lparam: LPARAM) -> ScreenPoint {
    let packed = lparam as u32;
    let mut point = POINT {
        x: (packed as u16 as i16) as i32,
        y: (((packed >> 16) as u16) as i16) as i32,
    };
    // The original application deliberately ignores the BOOL result and
    // returns the native output (or the unchanged input).
    unsafe {
        ClientToScreen(hwnd, &mut point);
    }
    ScreenPoint::new(point.x, point.y)
}
