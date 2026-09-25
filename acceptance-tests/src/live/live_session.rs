use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use std::time::Duration;

use frigotab::key_handling::KeyHandling;
use frigotab::keyboard_input::{KeyTransition, KeyboardInput, SwitcherKey};
use frigotab::screen_point::ScreenPoint;
use frigotab::session_window::{BackgroundMode, SessionWindow, WM_DESKTOP_SNAPSHOT_READY};
use frigotab::switcher_application::{AltTabBehavior, SwitcherApplication};
use frigotab::switcher_state::SwitcherState;
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
    GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, IDC_ARROW, IDI_APPLICATION, IsWindow,
    LoadCursorW, LoadIconW, MA_NOACTIVATE, RegisterClassExW, SetWindowLongPtrW, WM_CLOSE,
    WM_ERASEBKGND, WM_MOUSEACTIVATE, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WNDCLASSEXW,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use super::live_session_state::LiveSessionState;
use crate::{
    FixtureOptions, FixtureWindow, GREEN, LiveTile, MAGENTA, ORANGE, foreground_window,
    is_window_visible, pump_messages, use_per_monitor_physical_coordinates, wait_until, wide,
};

pub struct LiveSession {
    state: Box<LiveSessionState>,
    owner: HWND,
    fixtures: Vec<FixtureWindow>,
    previous_foreground: HWND,
}

impl LiveSession {
    pub fn new() -> Self {
        use_per_monitor_physical_coordinates();
        register_live_owner_class();

        let previous_foreground = foreground_window();
        let mut state = Box::new(LiveSessionState {
            controller: SwitcherApplication::new(),
            session: None,
        });
        let state_pointer = (&mut *state) as *mut LiveSessionState;
        let class_name = live_owner_class_name();
        let title = wide("FrigoTab acceptance session");
        let owner = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                state_pointer.cast(),
            )
        };
        assert!(
            !owner.is_null(),
            "CreateWindowExW failed for live session owner"
        );

        state.session = Some(SessionWindow::new(owner));

        let colors = [MAGENTA, GREEN, ORANGE];
        let mut fixtures = Vec::with_capacity(colors.len());
        for (index, color) in colors.into_iter().enumerate() {
            let bounds = RECT {
                left: 80 + index as i32 * 90,
                top: 80 + index as i32 * 70,
                right: 80 + index as i32 * 90 + 640,
                bottom: 80 + index as i32 * 70 + 480,
            };
            fixtures.push(FixtureWindow::show_with_options(
                &format!("FrigoTab real acceptance window {}", index + 1),
                color,
                FixtureOptions {
                    bounds,
                    activate: true,
                    ..FixtureOptions::default()
                },
            ));
        }
        pump_messages();

        Self {
            state,
            owner,
            fixtures,
            previous_foreground,
        }
    }

    pub fn owner(&self) -> HWND {
        self.owner
    }

    pub fn fixtures(&self) -> &[FixtureWindow] {
        &self.fixtures
    }

    pub fn close_fixture(&mut self, handle: HWND) -> bool {
        let Some(index) = self
            .fixtures
            .iter()
            .position(|fixture| fixture.handle() == handle)
        else {
            return false;
        };
        let mut fixture = self.fixtures.remove(index);
        fixture.close();
        true
    }

    pub fn fixture_handles(&self) -> Vec<HWND> {
        self.fixtures.iter().map(FixtureWindow::handle).collect()
    }

    pub fn is_visible(&self) -> bool {
        is_window_visible(self.owner) && self.state.controller.state() == SwitcherState::Visible
    }

    pub fn switcher_state(&self) -> SwitcherState {
        self.state.controller.state()
    }

    pub fn set_alt_tab_behavior(&mut self, behavior: AltTabBehavior) {
        self.state.controller.set_alt_tab_behavior(behavior);
    }

    pub fn alt_tab_behavior(&self) -> AltTabBehavior {
        self.state.controller.alt_tab_behavior()
    }

    pub fn set_background_mode(&mut self, mode: BackgroundMode) {
        self.state
            .session
            .as_mut()
            .expect("live session exists")
            .set_background_mode(mode);
        pump_messages();
    }

    pub fn background_mode(&self) -> BackgroundMode {
        self.state
            .session
            .as_ref()
            .expect("live session exists")
            .background_mode()
    }

    pub fn set_close_buttons_visible(&mut self, visible: bool) {
        self.state
            .session
            .as_mut()
            .expect("live session exists")
            .set_close_buttons_visible(visible);
        pump_messages();
    }

    pub fn close_buttons_visible(&self) -> bool {
        self.state
            .session
            .as_ref()
            .expect("live session exists")
            .close_buttons_visible()
    }

    pub fn candidate_count(&self) -> usize {
        self.state.controller.candidate_count()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.controller.selected_index()
    }

    pub fn selected_count(&self) -> usize {
        self.state
            .session
            .as_ref()
            .and_then(SessionWindow::applications)
            .map_or(0, |applications| {
                applications
                    .windows()
                    .iter()
                    .filter(|window| window.is_selected())
                    .count()
            })
    }

    pub fn tiles(&self) -> Vec<LiveTile> {
        self.state
            .session
            .as_ref()
            .and_then(SessionWindow::applications)
            .map(|applications| {
                applications
                    .windows()
                    .iter()
                    .map(|window| LiveTile {
                        source: window.application(),
                        popup: window.hwnd(),
                        bounds: window.bounds(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn selected_source(&self) -> Option<HWND> {
        let applications = self
            .state
            .session
            .as_ref()
            .and_then(SessionWindow::applications)?;
        let index = applications.selected_index()?;
        applications
            .windows()
            .get(index)
            .map(|window| window.application())
    }

    pub fn selected_tile(&self) -> Option<LiveTile> {
        let source = self.selected_source()?;
        self.tiles().into_iter().find(|tile| tile.source == source)
    }

    pub fn open(&mut self) -> KeyHandling {
        self.key(KeyboardInput::new(
            SwitcherKey::Tab,
            KeyTransition::Down,
            true,
            false,
            false,
        ))
    }

    pub fn key(&mut self, input: KeyboardInput) -> KeyHandling {
        let result = {
            let state = &mut *self.state;
            state
                .controller
                .handle_keyboard(state.session.as_mut().expect("live session exists"), input)
        };
        pump_messages();
        result
    }

    pub fn mouse_move(&mut self, point: ScreenPoint) {
        let state = &mut *self.state;
        state
            .controller
            .handle_mouse_move(state.session.as_mut().expect("live session exists"), point);
        pump_messages();
    }

    pub fn mouse_click(&mut self, point: ScreenPoint) {
        let state = &mut *self.state;
        state
            .controller
            .handle_mouse_click(state.session.as_mut().expect("live session exists"), point);
        pump_messages();
    }

    pub fn close(&mut self) {
        let state = &mut *self.state;
        state
            .controller
            .close(state.session.as_mut().expect("live session exists"));
        pump_messages();
    }

    pub fn wait_visible(&mut self, timeout: Duration) -> bool {
        wait_until(timeout, || self.is_visible())
    }

    pub fn wait_hidden(&mut self, timeout: Duration) -> bool {
        wait_until(timeout, || !self.is_visible())
    }
}

impl Default for LiveSession {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LiveSession {
    fn drop(&mut self) {
        if !self.owner.is_null() && unsafe { IsWindow(self.owner) } != 0 {
            self.close();
            if let Some(session) = self.state.session.as_mut() {
                session.dispose();
            }
            unsafe { DestroyWindow(self.owner) };
        } else if let Some(session) = self.state.session.as_mut() {
            session.dispose();
        }
        for fixture in &mut self.fixtures {
            fixture.close();
        }
        if !self.previous_foreground.is_null() {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(
                    self.previous_foreground,
                );
            }
        }
        pump_messages();
    }
}

fn live_owner_class_name() -> &'static Vec<u16> {
    static NAME: OnceLock<Vec<u16>> = OnceLock::new();
    NAME.get_or_init(|| wide("FrigoTab.AcceptanceLiveSession"))
}

fn register_live_owner_class() {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| unsafe {
        let class_name = live_owner_class_name();
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(live_owner_window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(null()),
            hIcon: LoadIconW(null_mut(), IDI_APPLICATION),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: LoadIconW(null_mut(), IDI_APPLICATION),
        };
        let _ = RegisterClassExW(&class);
    });
}

unsafe extern "system" fn live_owner_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    if message == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        if !create.is_null() {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as isize);
            }
            return 1;
        }
    }

    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut LiveSessionState };
    if pointer.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }

    let state = unsafe { &mut *pointer };
    match message {
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = unsafe { BeginPaint(hwnd, &mut paint) };
            let mut client = RECT::default();
            unsafe {
                GetClientRect(hwnd, &mut client);
            }
            if let Some(session) = state.session.as_ref() {
                session.paint(dc, client);
            }
            unsafe {
                EndPaint(hwnd, &paint);
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        WM_DESKTOP_SNAPSHOT_READY => {
            if let Some(session) = state.session.as_mut() {
                session.publish_desktop_snapshot();
            }
            0
        }
        WM_CLOSE => {
            state
                .controller
                .close(state.session.as_mut().expect("live session exists"));
            state
                .session
                .as_mut()
                .expect("live session exists")
                .dispose();
            unsafe {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_NCDESTROY => unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, message, wparam, lparam)
        },
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
