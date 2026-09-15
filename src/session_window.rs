//! Native session-window adapter corresponding to the original session form.
//!
//! The controller remains unaware of HWNDs and DWM.  This type owns the one
//! full-desktop owner window, the retained Explorer frame, and the current
//! `ApplicationWindows` collection. The order in `try_open` is intentional:
//! prepare DWM previews behind the hidden owner, show and synchronously complete
//! the owner's paint cycle, reveal the layered overlays, then request foreground
//! activation.

use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BLACKNESS, GetDC, HDC, InvalidateRect, PaintDesktop, PatBlt, ReleaseDC, UpdateWindow,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClientRect, HWND_TOPMOST, PostMessageW, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_NOOWNERZORDER,
    SetWindowPos, ShowWindow, WM_APP,
};

use crate::application_windows::ApplicationWindows;
use crate::rect::virtual_screen_bounds;
use crate::screen_point::ScreenPoint;
use crate::shell_desktop_snapshot::ShellDesktopSnapshot;
use crate::switcher_application::SwitcherSessionPort;
use crate::window_finder::WindowFinder;
use crate::window_handle::WindowHandle;

pub const WM_DESKTOP_SNAPSHOT_READY: u32 = WM_APP + 3;

/// Controls what SessionWindow paints behind the application previews.
///
/// The full Explorer desktop (including icons) is the historical behavior and
/// remains the default. `ImageOnly` asks User32 to paint only the desktop
/// pattern or wallpaper, while `Black` deliberately avoids touching the shell
/// or any screen DC.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BackgroundMode {
    #[default]
    FullDesktop,
    ImageOnly,
    Black,
}

struct BackgroundState {
    mode: BackgroundMode,
    desktop_snapshot: Option<ShellDesktopSnapshot>,
    desktop_snapshot_bounds: RECT,
    owner_bounds: RECT,
    disposed: bool,
}

/// A cloneable, UI-thread-only view of the owner window's paint state.
///
/// Win32 can synchronously deliver `WM_PAINT` from inside `UpdateWindow`.
/// Keeping this state in its own `Rc<RefCell<_>>` lets the WndProc repaint
/// without aliasing the mutable `App`/`SessionWindow` borrow which initiated
/// the native call.
#[derive(Clone)]
pub struct SessionPainter(Rc<RefCell<BackgroundState>>);

impl SessionPainter {
    fn new(desktop_snapshot: Option<ShellDesktopSnapshot>, desktop_snapshot_bounds: RECT) -> Self {
        Self(Rc::new(RefCell::new(BackgroundState {
            mode: BackgroundMode::FullDesktop,
            desktop_snapshot,
            desktop_snapshot_bounds,
            owner_bounds: RECT::default(),
            disposed: false,
        })))
    }

    pub fn mode(&self) -> BackgroundMode {
        self.0.borrow().mode
    }

    fn set_mode(&self, mode: BackgroundMode) {
        self.0.borrow_mut().mode = mode;
    }

    fn has_snapshot_for(&self, bounds: RECT) -> bool {
        let state = self.0.borrow();
        state.desktop_snapshot.is_some() && same_rect(state.desktop_snapshot_bounds, bounds)
    }

    fn set_owner_bounds(&self, bounds: RECT) {
        self.0.borrow_mut().owner_bounds = bounds;
    }

    fn publish_snapshot(&self, snapshot: ShellDesktopSnapshot, bounds: RECT) {
        let mut state = self.0.borrow_mut();
        state.desktop_snapshot = Some(snapshot);
        state.desktop_snapshot_bounds = bounds;
    }

    fn dispose(&self) {
        let mut state = self.0.borrow_mut();
        state.disposed = true;
        state.desktop_snapshot = None;
        state.desktop_snapshot_bounds = RECT::default();
    }

    /// Paint the configured background into an owner-window client DC.
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HDC is an opaque GDI handle.
    pub fn paint(&self, destination_dc: HDC, destination_bounds: RECT) {
        let Ok(state) = self.0.try_borrow() else {
            return;
        };
        if state.disposed
            || destination_bounds.right <= destination_bounds.left
            || destination_bounds.bottom <= destination_bounds.top
        {
            return;
        }
        match state.mode {
            BackgroundMode::FullDesktop => {
                if let Some(snapshot) = state
                    .desktop_snapshot
                    .as_ref()
                    .filter(|_| same_rect(state.desktop_snapshot_bounds, state.owner_bounds))
                {
                    snapshot.draw(destination_dc, destination_bounds);
                } else {
                    paint_black(destination_dc, destination_bounds);
                }
            }
            BackgroundMode::ImageOnly => {
                if unsafe { PaintDesktop(destination_dc) } == 0 {
                    paint_black(destination_dc, destination_bounds);
                }
            }
            BackgroundMode::Black => paint_black(destination_dc, destination_bounds),
        }
    }
}

struct SnapshotCompletion {
    snapshot: ShellDesktopSnapshot,
    bounds: RECT,
}

struct WindowDc {
    owner: HWND,
    handle: NonNull<c_void>,
}

impl WindowDc {
    fn acquire(owner: HWND) -> Option<Self> {
        NonNull::new(unsafe { GetDC(owner) }).map(|handle| Self { owner, handle })
    }

    fn handle(&self) -> HDC {
        self.handle.as_ptr()
    }
}

impl Drop for WindowDc {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(self.owner, self.handle());
        }
    }
}

/// The Win32 side of a live switcher session.
pub struct SessionWindow {
    hwnd: HWND,
    painter: SessionPainter,
    owner_bounds: RECT,
    applications: Option<ApplicationWindows>,
    snapshot_completion: Arc<Mutex<Option<SnapshotCompletion>>>,
    snapshot_refresh_running: Arc<AtomicBool>,
    disposal_requested: Arc<AtomicBool>,
    snapshot_worker: Option<JoinHandle<()>>,
    activating_selection: bool,
    disposed: bool,
}

impl SessionWindow {
    /// Create the hidden owner and capture the shell frame before the hook is
    /// installed.  The caller creates/registers the HWND so its WndProc can
    /// be installed in the same way as the original `FrigoForm`.
    pub fn new(hwnd: HWND) -> Self {
        let bounds = virtual_screen_bounds();
        let snapshot = ShellDesktopSnapshot::capture(bounds);
        let snapshot_available = snapshot.is_available();
        let painter = SessionPainter::new(
            snapshot_available.then_some(snapshot),
            if snapshot_available {
                bounds
            } else {
                RECT::default()
            },
        );
        Self {
            hwnd,
            painter,
            owner_bounds: RECT::default(),
            applications: None,
            snapshot_completion: Arc::new(Mutex::new(None)),
            snapshot_refresh_running: Arc::new(AtomicBool::new(false)),
            disposal_requested: Arc::new(AtomicBool::new(false)),
            snapshot_worker: None,
            activating_selection: false,
            disposed: false,
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn bounds(&self) -> RECT {
        self.owner_bounds
    }

    pub fn background_mode(&self) -> BackgroundMode {
        self.painter.mode()
    }

    pub fn painter(&self) -> SessionPainter {
        self.painter.clone()
    }

    /// Change the backdrop without disturbing the current preview graph.
    ///
    /// The inexpensive User32/black modes repaint immediately and never start
    /// a shell-capture worker. Switching back to the full desktop reuses a
    /// retained capture when possible and otherwise queues exactly one refresh.
    pub fn set_background_mode(&mut self, mode: BackgroundMode) {
        if self.background_mode() == mode || self.disposed {
            return;
        }
        self.painter.set_mode(mode);
        if mode == BackgroundMode::FullDesktop {
            let bounds = virtual_screen_bounds();
            // Refresh while idle even when a matching retained frame exists:
            // ImageOnly/Black may have been selected long enough for the
            // wallpaper or desktop icons to change.
            if self.applications.is_none() {
                self.queue_desktop_snapshot_refresh(bounds);
            }
        }
        self.invalidate_background();
    }

    pub fn applications(&self) -> Option<&ApplicationWindows> {
        self.applications.as_ref()
    }

    /// True only while foreground activation is being handed to the selected
    /// application.  WM_ACTIVATEAPP(FALSE) during this small window is the
    /// expected handoff and must not interrupt the session.
    pub fn activating_selection(&self) -> bool {
        self.activating_selection
    }

    /// Paint the selected background through the reentrancy-safe painter.
    pub fn paint(&self, destination_dc: HDC, destination_bounds: RECT) {
        self.painter.paint(destination_dc, destination_bounds);
    }

    /// Best-effort destruction of the session resources.  The controller may
    /// call this repeatedly while handling a failed open or interruption.
    pub fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposal_requested.store(true, Ordering::Release);
        self.disposed = true;
        self.close_session_resources();
        // The completion mutex is also the worker's finalization gate. Once
        // this lock is acquired after cancellation, the worker has either
        // posted while the HWND is still owned or has observed cancellation.
        self.snapshot_completion
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        // A hung Explorer PrintWindow must not hang UI teardown. The worker's
        // Arc-owned state remains valid until the detached thread returns.
        drop(self.snapshot_worker.take());
        self.painter.dispose();
    }

    fn try_open_session(&mut self) -> Result<usize, ()> {
        if self.disposed {
            return Err(());
        }

        self.close_session_resources();
        let finder = WindowFinder::new();
        if finder.windows.is_empty() {
            return Ok(0);
        }

        let bounds = virtual_screen_bounds();
        if bounds.right <= bounds.left || bounds.bottom <= bounds.top {
            return Ok(0);
        }
        self.set_owner_bounds(bounds);

        if self.background_mode() == BackgroundMode::FullDesktop
            && !self.painter.has_snapshot_for(bounds)
        {
            self.queue_desktop_snapshot_refresh(bounds);
        }

        let owner = WindowHandle::new(self.hwnd);
        let applications = ApplicationWindows::new(owner, &finder);
        if applications.is_empty() {
            return Ok(0);
        }

        // Publish only after every candidate has been constructed. DWM has
        // prepared the thumbnails behind this still-hidden owner.
        self.applications = Some(applications);
        unsafe {
            ShowWindow(self.hwnd, SW_SHOW);
        }
        self.paint_owner_synchronously();
        if let Some(applications) = self.applications.as_mut() {
            applications.set_visible(true);
        }
        // Foreground activation is deliberately last; a SetForegroundWindow
        // deactivation is the expected selection handoff, not interruption.
        let _ = owner.set_foreground();
        Ok(self
            .applications
            .as_ref()
            .map_or(0, ApplicationWindows::count))
    }

    fn close_session_resources(&mut self) {
        let mut applications = self.applications.take();
        if let Some(current) = applications.as_mut() {
            current.set_visible(false);
        }
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
        if let Some(current) = applications.as_mut() {
            current.dispose();
        }
        if applications.is_some() && !self.disposed {
            self.queue_desktop_snapshot_refresh_current();
        }
    }

    /// Queue the same idle Explorer refresh performed by SessionForm after a
    /// session closes or the display/compositor topology changes.
    pub fn queue_desktop_snapshot_refresh_current(&mut self) {
        self.queue_desktop_snapshot_refresh(virtual_screen_bounds());
    }

    fn queue_desktop_snapshot_refresh(&mut self, bounds: RECT) {
        self.reap_finished_snapshot_worker();
        if self.background_mode() != BackgroundMode::FullDesktop
            || self.disposed
            || bounds.right <= bounds.left
            || bounds.bottom <= bounds.top
            || self
                .snapshot_refresh_running
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return;
        }

        let completion = Arc::clone(&self.snapshot_completion);
        let running = Arc::clone(&self.snapshot_refresh_running);
        let disposed = Arc::clone(&self.disposal_requested);
        let owner = self.hwnd as usize;
        match std::thread::Builder::new()
            .name("FrigoTab desktop snapshot".to_string())
            .spawn(move || {
                let candidate = ShellDesktopSnapshot::capture(bounds);
                if !candidate.is_available() {
                    running.store(false, Ordering::Release);
                    return;
                }

                let mut slot = completion.lock().unwrap_or_else(|error| error.into_inner());
                if disposed.load(Ordering::Acquire) {
                    running.store(false, Ordering::Release);
                    return;
                }
                *slot = Some(SnapshotCompletion {
                    snapshot: candidate,
                    bounds,
                });

                if unsafe { PostMessageW(owner as HWND, WM_DESKTOP_SNAPSHOT_READY, 0, 0) } == 0 {
                    *slot = None;
                    running.store(false, Ordering::Release);
                }
            }) {
            Ok(worker) => self.snapshot_worker = Some(worker),
            Err(_) => {
                self.snapshot_refresh_running
                    .store(false, Ordering::Release);
            }
        }
    }

    /// Publish a completed worker capture on the UI thread. A capture that
    /// finishes while previews are visible is discarded, preserving one
    /// stable background for the whole session.
    pub fn publish_desktop_snapshot(&mut self) {
        self.snapshot_refresh_running
            .store(false, Ordering::Release);
        self.reap_finished_snapshot_worker();
        let Some(completion) = self
            .snapshot_completion
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        else {
            return;
        };
        if self.disposed || self.applications.is_some() {
            return;
        }

        let current_bounds = virtual_screen_bounds();
        if !same_rect(current_bounds, completion.bounds) {
            if self.background_mode() == BackgroundMode::FullDesktop {
                self.queue_desktop_snapshot_refresh(current_bounds);
            }
            return;
        }

        self.painter
            .publish_snapshot(completion.snapshot, completion.bounds);
        if self.background_mode() == BackgroundMode::FullDesktop {
            self.invalidate_background();
        }
    }

    fn reap_finished_snapshot_worker(&mut self) {
        if !self.snapshot_refresh_running.load(Ordering::Acquire) {
            self.join_snapshot_worker();
        }
    }

    fn join_snapshot_worker(&mut self) {
        if let Some(worker) = self.snapshot_worker.take() {
            let _ = worker.join();
        }
    }

    fn invalidate_background(&self) {
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    fn paint_owner_synchronously(&self) {
        let mut client = RECT::default();
        if unsafe { GetClientRect(self.hwnd, &mut client) } == 0 {
            return;
        }
        {
            let Some(dc) = WindowDc::acquire(self.hwnd) else {
                return;
            };
            self.paint(dc.handle(), client);
        }
        unsafe {
            // This synchronously validates/publishes the direct first-frame
            // backdrop before the prepared previews and layered overlays are
            // composed. A reentrant WM_PAINT uses the separately shared
            // SessionPainter, never the App or this mutably borrowed
            // SessionWindow.
            UpdateWindow(self.hwnd);
        }
    }

    fn set_owner_bounds(&mut self, bounds: RECT) {
        self.owner_bounds = bounds;
        self.painter.set_owner_bounds(bounds);
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                bounds.left,
                bounds.top,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
        }
    }

    /// A display/DPI change invalidates the native preview graph. The original
    /// implementation deliberately closes rather than showing stale tiles.
    fn relayout_session(&mut self) -> Result<(), ()> {
        if self.applications.is_some() {
            return Err(());
        }
        Ok(())
    }
}

impl SwitcherSessionPort for SessionWindow {
    fn try_open(&mut self) -> Result<usize, ()> {
        self.try_open_session()
    }

    fn select(&mut self, index: usize) -> Result<(), ()> {
        let Some(applications) = self.applications.as_mut() else {
            return Err(());
        };
        applications.select_by_index(Some(index)).map_err(|_| ())
    }

    fn clear_selection(&mut self) -> Result<(), ()> {
        let Some(applications) = self.applications.as_mut() else {
            return Err(());
        };
        applications.select_by_index(None).map_err(|_| ())
    }

    fn hit_test(&mut self, point: ScreenPoint) -> Result<Option<usize>, ()> {
        let Some(applications) = self.applications.as_ref() else {
            return Err(());
        };
        Ok(applications.hit_test(point.x, point.y))
    }

    fn try_activate_selected(&mut self) -> Result<bool, ()> {
        let Some(applications) = self.applications.as_ref() else {
            return Ok(false);
        };
        self.activating_selection = true;
        let result = applications.try_activate_selected();
        self.activating_selection = false;
        Ok(result)
    }

    fn close(&mut self) {
        if !self.disposed {
            self.close_session_resources();
        }
    }

    fn relayout(&mut self) -> Result<(), ()> {
        self.relayout_session()
    }
}

impl Drop for SessionWindow {
    fn drop(&mut self) {
        self.dispose();
    }
}

fn same_rect(left: RECT, right: RECT) -> bool {
    left.left == right.left
        && left.top == right.top
        && left.right == right.right
        && left.bottom == right.bottom
}

fn paint_black(destination_dc: HDC, destination_bounds: RECT) {
    if destination_dc.is_null()
        || destination_bounds.right <= destination_bounds.left
        || destination_bounds.bottom <= destination_bounds.top
    {
        return;
    }
    unsafe {
        PatBlt(
            destination_dc,
            destination_bounds.left,
            destination_bounds.top,
            destination_bounds.right - destination_bounds.left,
            destination_bounds.bottom - destination_bounds.top,
            BLACKNESS,
        );
    }
}
