#![cfg(windows)]

//! Real Win32 drivers shared by the acceptance tests.
//!
//! This module contains real fixture HWNDs, a child process running the real
//! executable, bounded native polling, and the production objects needed by
//! the in-process scenarios. It has no substitute session or fake window API.

use std::ffi::OsStr;
use std::io;
use std::iter::once;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus};
use std::ptr::{null, null_mut};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use frigotab::key_handling::KeyHandling;
use frigotab::keyboard_input::{KeyTransition, KeyboardInput, SwitcherKey};
use frigotab::screen_point::ScreenPoint;
use frigotab::session_window::{SessionWindow, WM_DESKTOP_SNAPSHOT_READY};
use frigotab::switcher_application::SwitcherApplication;
use frigotab::switcher_state::SwitcherState;
use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BeginPaint, BitBlt, CreateCompatibleDC, CreateDIBSection,
    CreateSolidBrush, DIB_RGB_COLORS, DeleteDC, DeleteObject, EndPaint, FillRect, GetDC, GetPixel,
    HGDIOBJ, PAINTSTRUCT, ReleaseDC, SRCCOPY, SelectObject, UpdateWindow,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, EnumWindows, GW_OWNER, GWL_EXSTYLE, GWL_STYLE, GWLP_USERDATA, GetClientRect,
    GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IDC_ARROW, IDI_APPLICATION, IsWindow,
    IsWindowVisible, LoadCursorW, LoadIconW, MA_NOACTIVATE, MSG, PM_REMOVE, PeekMessageW,
    PostMessageW, RegisterClassExW, SW_HIDE, SW_MINIMIZE, SW_RESTORE, SW_SHOWNA, SWP_NOACTIVATE,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow,
    TranslateMessage, WM_CLOSE, WM_ERASEBKGND, WM_MOUSEACTIVATE, WM_NCCREATE, WM_NCDESTROY,
    WM_PAINT, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW,
    WS_POPUP,
};

pub type Color = u32;
pub const RED: Color = rgb(255, 0, 0);
pub const GREEN: Color = rgb(0, 255, 0);
pub const BLUE: Color = rgb(0, 0, 255);
pub const MAGENTA: Color = rgb(255, 0, 255);
pub const ORANGE: Color = rgb(255, 165, 0);

pub const fn rgb(red: u8, green: u8, blue: u8) -> Color {
    red as Color | ((green as Color) << 8) | ((blue as Color) << 16)
}

pub fn serial_guard() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn set_per_monitor_dpi_awareness() -> bool {
    unsafe { !SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).is_null() }
}

pub fn use_per_monitor_physical_coordinates() -> bool {
    set_per_monitor_dpi_awareness()
}

#[derive(Clone, Copy)]
pub struct FixtureOptions {
    pub bounds: RECT,
    pub style: u32,
    pub ex_style: u32,
    pub activate: bool,
    pub initially_visible: bool,
}

impl Default for FixtureOptions {
    fn default() -> Self {
        Self {
            bounds: RECT {
                left: 100,
                top: 100,
                right: 500,
                bottom: 400,
            },
            style: WS_OVERLAPPEDWINDOW,
            ex_style: 0,
            activate: false,
            initially_visible: true,
        }
    }
}

pub struct FixtureWindow {
    hwnd: HWND,
}

impl FixtureWindow {
    pub fn show(title: &str, color: Color, bounds: RECT) -> Self {
        Self::show_with_options(
            title,
            color,
            FixtureOptions {
                bounds,
                ..Default::default()
            },
        )
    }

    pub fn show_with_options(title: &str, color: Color, options: FixtureOptions) -> Self {
        register_fixture_class();
        let title = wide(title);
        let hwnd = unsafe {
            CreateWindowExW(
                options.ex_style,
                fixture_class_name().as_ptr(),
                title.as_ptr(),
                options.style,
                options.bounds.left,
                options.bounds.top,
                options.bounds.right.saturating_sub(options.bounds.left),
                options.bounds.bottom.saturating_sub(options.bounds.top),
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                color as usize as *mut _,
            )
        };
        assert!(!hwnd.is_null(), "CreateWindowExW failed for fixture window");
        if options.initially_visible {
            unsafe {
                ShowWindow(
                    hwnd,
                    if options.activate {
                        SW_RESTORE
                    } else {
                        SW_SHOWNA
                    },
                );
                if options.activate {
                    SetForegroundWindow(hwnd);
                }
                UpdateWindow(hwnd);
            };
        }
        pump_messages();
        Self { hwnd }
    }

    pub fn handle(&self) -> HWND {
        self.hwnd
    }

    pub fn bounds(&self) -> RECT {
        window_bounds(self.hwnd).unwrap_or_default()
    }

    pub fn set_bounds(&self, bounds: RECT) -> bool {
        if self.hwnd.is_null() {
            return false;
        }
        let ok = unsafe {
            SetWindowPos(
                self.hwnd,
                null_mut(),
                bounds.left,
                bounds.top,
                bounds.right.saturating_sub(bounds.left),
                bounds.bottom.saturating_sub(bounds.top),
                SWP_NOACTIVATE,
            ) != 0
        };
        pump_messages();
        ok
    }

    pub fn title(&self) -> String {
        window_title(self.hwnd)
    }

    pub fn set_title(&self, title: &str) -> bool {
        let title = wide(title);
        unsafe { SetWindowTextW(self.hwnd, title.as_ptr()) != 0 }
    }

    pub fn hide(&self) {
        unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        pump_messages();
    }

    pub fn show_again(&self) {
        unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        pump_messages();
    }

    pub fn minimize(&self) {
        unsafe { ShowWindow(self.hwnd, SW_MINIMIZE) };
        pump_messages();
    }

    pub fn restore(&self) {
        unsafe { ShowWindow(self.hwnd, SW_RESTORE) };
        pump_messages();
    }

    pub fn close(&mut self) {
        self.destroy();
    }

    fn destroy(&mut self) {
        if !self.hwnd.is_null() && unsafe { IsWindow(self.hwnd) } != 0 {
            unsafe { DestroyWindow(self.hwnd) };
            pump_messages();
        }
        self.hwnd = null_mut();
    }
}

impl Drop for FixtureWindow {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// A source/popup pair observed from one in-process switcher session.
///
/// The source is the real fixture HWND selected by the production finder; the
/// popup is the real layered preview HWND created by `ApplicationWindow`.
#[derive(Clone, Copy)]
pub struct LiveTile {
    pub source: HWND,
    pub popup: HWND,
    pub bounds: RECT,
}

struct LiveSessionState {
    controller: SwitcherApplication,
    session: Option<SessionWindow>,
}

/// Real in-process acceptance harness for the production switcher objects.
///
/// This is deliberately not a substitute session port.  `SessionWindow` owns
/// the native owner, DWM thumbnails, layered popups, and shell frame while
/// `SwitcherApplication` receives the same normalized keyboard and pointer
/// inputs that the production hook/owner adapter supplies.
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

impl Drop for LiveSession {
    fn drop(&mut self) {
        self.close();
        if !self.owner.is_null() && unsafe { IsWindow(self.owner) } != 0 {
            unsafe { DestroyWindow(self.owner) };
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
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("FrigoTab exited before creating its owner window ({status})"),
                ));
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

pub struct ScreenCapture {
    bounds: RECT,
    pixels: Vec<Color>,
}

impl ScreenCapture {
    pub fn bounds(&self) -> RECT {
        self.bounds
    }
    pub fn width(&self) -> i32 {
        self.bounds.right - self.bounds.left
    }
    pub fn height(&self) -> i32 {
        self.bounds.bottom - self.bounds.top
    }
    pub fn pixel(&self, x: i32, y: i32) -> Option<Color> {
        if x < self.bounds.left
            || x >= self.bounds.right
            || y < self.bounds.top
            || y >= self.bounds.bottom
        {
            return None;
        }
        let index = (y - self.bounds.top) as usize * self.width() as usize
            + (x - self.bounds.left) as usize;
        self.pixels.get(index).copied()
    }
    pub fn find(&self, expected: Color, tolerance: u32) -> Option<POINT> {
        for y in self.bounds.top..self.bounds.bottom {
            for x in self.bounds.left..self.bounds.right {
                if self
                    .pixel(x, y)
                    .is_some_and(|color| color_distance(color, expected) <= tolerance)
                {
                    return Some(POINT { x, y });
                }
            }
        }
        None
    }
}

pub fn capture_screen_image(bounds: RECT) -> Option<ScreenCapture> {
    let width = bounds.right.saturating_sub(bounds.left);
    let height = bounds.bottom.saturating_sub(bounds.top);
    if width <= 0 || height <= 0 {
        return None;
    }
    let screen = unsafe { GetDC(null_mut()) };
    if screen.is_null() {
        return None;
    }
    let memory = unsafe { CreateCompatibleDC(screen) };
    if memory.is_null() {
        unsafe { ReleaseDC(null_mut(), screen) };
        return None;
    }
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            ..unsafe { zeroed() }
        },
        ..unsafe { zeroed() }
    };
    let mut bits = null_mut();
    let bitmap =
        unsafe { CreateDIBSection(screen, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0) };
    if bitmap.is_null() || bits.is_null() {
        if !bitmap.is_null() {
            unsafe { DeleteObject(bitmap) };
        }
        unsafe {
            DeleteDC(memory);
            ReleaseDC(null_mut(), screen);
        }
        return None;
    }
    let previous = unsafe { SelectObject(memory, bitmap as HGDIOBJ) };
    let copied = unsafe {
        BitBlt(
            memory,
            0,
            0,
            width,
            height,
            screen,
            bounds.left,
            bounds.top,
            SRCCOPY,
        )
    };
    let pixels = if copied != 0 {
        unsafe { std::slice::from_raw_parts(bits as *const Color, (width * height) as usize) }
            .iter()
            .map(|pixel| {
                let red = (pixel >> 16) & 0xff;
                let green = (pixel >> 8) & 0xff;
                let blue = pixel & 0xff;
                red | (green << 8) | (blue << 16)
            })
            .collect()
    } else {
        Vec::new()
    };
    unsafe {
        if !previous.is_null() {
            SelectObject(memory, previous);
        }
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(null_mut(), screen);
    }
    (copied != 0).then_some(ScreenCapture { bounds, pixels })
}

pub fn capture_screen(bounds: RECT) -> Option<Vec<Color>> {
    capture_screen_image(bounds).map(|capture| capture.pixels)
}

pub fn pixel_at(x: i32, y: i32) -> Option<Color> {
    unsafe {
        let dc = GetDC(null_mut());
        if dc.is_null() {
            return None;
        }
        let color = GetPixel(dc, x, y);
        ReleaseDC(null_mut(), dc);
        (color != u32::MAX).then_some(color)
    }
}

pub fn screen_pixel(x: i32, y: i32) -> Option<Color> {
    pixel_at(x, y)
}
pub fn find_screen_pixel(bounds: RECT, expected: Color, tolerance: u32) -> Option<POINT> {
    capture_screen_image(bounds)?.find(expected, tolerance)
}
pub fn find_pixel(bounds: RECT, expected: Color, tolerance: u32) -> Option<POINT> {
    find_screen_pixel(bounds, expected, tolerance)
}

pub const fn color_distance(first: Color, second: Color) -> u32 {
    (first & 0xff).abs_diff(second & 0xff)
        + ((first >> 8) & 0xff).abs_diff((second >> 8) & 0xff)
        + ((first >> 16) & 0xff).abs_diff((second >> 16) & 0xff)
}

pub fn pump_messages() {
    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

pub fn wait_until<F>(timeout: Duration, mut condition: F) -> bool
where
    F: FnMut() -> bool,
{
    let deadline = Instant::now() + timeout;
    loop {
        pump_messages();
        if condition() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(5));
    }
}

pub fn window_bounds(hwnd: HWND) -> Option<RECT> {
    if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
        return None;
    }
    let mut bounds = RECT::default();
    (unsafe { GetWindowRect(hwnd, &mut bounds) } != 0
        && bounds.right > bounds.left
        && bounds.bottom > bounds.top)
        .then_some(bounds)
}
pub fn get_window_rect(hwnd: HWND) -> Option<RECT> {
    window_bounds(hwnd)
}

pub fn window_title(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    unsafe {
        let length = GetWindowTextLengthW(hwnd);
        if length <= 0 {
            return String::new();
        }
        let mut text = vec![0u16; length as usize + 1];
        let copied = GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32);
        String::from_utf16_lossy(&text[..copied.max(0) as usize])
    }
}
pub fn is_window_visible(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsWindowVisible(hwnd) != 0 }
}
pub fn window_style(hwnd: HWND) -> u32 {
    unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 }
}
pub fn window_ex_style(hwnd: HWND) -> u32 {
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 }
}
pub fn window_owner(hwnd: HWND) -> HWND {
    unsafe { GetWindow(hwnd, GW_OWNER) }
}
pub fn foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}
pub fn is_layered(hwnd: HWND) -> bool {
    window_ex_style(hwnd) & WS_EX_LAYERED != 0
}

pub fn windows_for_pid(pid: u32) -> Vec<HWND> {
    let mut search = WindowSearch {
        pid,
        result: Vec::new(),
    };
    unsafe { EnumWindows(Some(enum_windows_callback), &mut search as *mut _ as LPARAM) };
    search.result
}
pub fn enumerate_windows(pid: u32) -> Vec<HWND> {
    windows_for_pid(pid)
}

pub fn visible_owned_layered_windows(owner: HWND) -> Vec<HWND> {
    let pid = window_pid(owner);
    windows_for_pid(pid)
        .into_iter()
        .filter(|&hwnd| {
            hwnd != owner
                && is_window_visible(hwnd)
                && is_layered(hwnd)
                && window_owner(hwnd) == owner
        })
        .collect()
}
pub fn visible_owned_overlays(owner: HWND) -> Vec<HWND> {
    visible_owned_layered_windows(owner)
}

pub fn find_window_by_pid_and_bounds(
    pid: u32,
    minimum_width: i32,
    minimum_height: i32,
) -> Option<HWND> {
    windows_for_pid(pid).into_iter().find(|&hwnd| {
        window_bounds(hwnd).is_some_and(|bounds| {
            bounds.right - bounds.left >= minimum_width
                && bounds.bottom - bounds.top >= minimum_height
        })
    })
}

struct WindowSearch {
    pid: u32,
    result: Vec<HWND>,
}
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> i32 {
    let search = unsafe { &mut *(lparam as *mut WindowSearch) };
    if window_pid(hwnd) == search.pid {
        search.result.push(hwnd);
    }
    1
}
fn window_pid(hwnd: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    pid
}

fn find_owner(pid: u32) -> Option<HWND> {
    windows_for_pid(pid).into_iter().find(|&hwnd| {
        window_ex_style(hwnd) & WS_EX_TOOLWINDOW != 0
            && window_bounds(hwnd).is_some_and(|bounds| {
                bounds.right - bounds.left >= 100 && bounds.bottom - bounds.top >= 100
            })
    })
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

fn register_fixture_class() {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| unsafe {
        let class_name = fixture_class_name();
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(fixture_window_proc),
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
fn fixture_class_name() -> &'static Vec<u16> {
    static NAME: OnceLock<Vec<u16>> = OnceLock::new();
    NAME.get_or_init(|| wide("FrigoTab.AcceptanceFixture"))
}

unsafe extern "system" fn fixture_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    if message == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        if !create.is_null() {
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as isize) };
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }
    }
    match message {
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
            let mut client = RECT::default();
            unsafe { GetClientRect(hwnd, &mut client) };
            let brush = unsafe { CreateSolidBrush(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as u32) };
            if !brush.is_null() {
                unsafe {
                    FillRect(hdc, &client, brush);
                    DeleteObject(brush)
                };
            }
            unsafe { EndPaint(hwnd, &paint) };
            0
        }
        WM_ERASEBKGND => 1,
        WM_NCDESTROY => unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, message, wparam, lparam)
        },
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}
