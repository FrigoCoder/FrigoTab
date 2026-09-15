//! Native session-window adapter corresponding to the original session form.
//!
//! The controller remains unaware of HWNDs and DWM.  This type owns the one
//! full-desktop owner window, the retained Explorer frame, and the current
//! `ApplicationWindows` collection.  The order in `try_open` is intentional:
//! construct hidden previews, show and synchronously paint the shell frame,
//! reveal thumbnails, then request foreground activation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{HDC, InvalidateRect, UpdateWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetSystemMetrics, HWND_TOPMOST, PostMessageW, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOW, SWP_NOACTIVATE,
    SWP_NOOWNERZORDER, SetWindowPos, ShowWindow, WM_APP,
};

use crate::application_windows::ApplicationWindows;
use crate::screen_point::ScreenPoint;
use crate::shell_desktop_snapshot::ShellDesktopSnapshot;
use crate::switcher_application::SwitcherSessionPort;
use crate::window_finder::WindowFinder;
use crate::window_handle::WindowHandle;

pub const WM_DESKTOP_SNAPSHOT_READY: u32 = WM_APP + 3;

struct SnapshotCompletion {
    snapshot: ShellDesktopSnapshot,
    bounds: RECT,
}

/// The Win32 side of a live switcher session.
pub struct SessionWindow {
    hwnd: HWND,
    desktop_snapshot: Option<ShellDesktopSnapshot>,
    desktop_snapshot_bounds: RECT,
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
        let bounds = virtual_bounds();
        let snapshot = ShellDesktopSnapshot::capture(bounds);
        let snapshot_available = snapshot.is_available();
        Self {
            hwnd,
            desktop_snapshot: snapshot_available.then_some(snapshot),
            desktop_snapshot_bounds: if snapshot_available {
                bounds
            } else {
                RECT::default()
            },
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

    pub fn applications(&self) -> Option<&ApplicationWindows> {
        self.applications.as_ref()
    }

    /// True only while foreground activation is being handed to the selected
    /// application.  WM_ACTIVATEAPP(FALSE) during this small window is the
    /// expected handoff and must not interrupt the session.
    pub fn activating_selection(&self) -> bool {
        self.activating_selection
    }

    /// Paint the retained shell image into the owner’s client DC.  The owner
    /// WndProc calls this from WM_PAINT before any DWM thumbnail is revealed.
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HDC is an opaque GDI handle.
    pub fn paint(&self, destination_dc: HDC, destination_bounds: RECT) {
        if self.disposed {
            return;
        }
        if destination_bounds.right <= destination_bounds.left
            || destination_bounds.bottom <= destination_bounds.top
        {
            return;
        }
        if let Some(snapshot) = self
            .desktop_snapshot
            .as_ref()
            .filter(|_| same_rect(self.desktop_snapshot_bounds, self.owner_bounds))
        {
            snapshot.draw(destination_dc, destination_bounds);
        } else {
            unsafe {
                windows_sys::Win32::Graphics::Gdi::PatBlt(
                    destination_dc,
                    destination_bounds.left,
                    destination_bounds.top,
                    destination_bounds.right - destination_bounds.left,
                    destination_bounds.bottom - destination_bounds.top,
                    windows_sys::Win32::Graphics::Gdi::BLACKNESS,
                );
            }
        }
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
        self.desktop_snapshot = None;
        self.desktop_snapshot_bounds = RECT::default();
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

        let bounds = virtual_bounds();
        if bounds.right <= bounds.left || bounds.bottom <= bounds.top {
            return Ok(0);
        }
        self.set_owner_bounds(bounds);

        if self.desktop_snapshot.is_none() || !same_rect(self.desktop_snapshot_bounds, bounds) {
            self.queue_desktop_snapshot_refresh(bounds);
        }

        let owner = WindowHandle::new(self.hwnd);
        let applications = ApplicationWindows::new(owner, &finder);
        if applications.is_empty() {
            return Ok(0);
        }

        // Publish only after every candidate has been constructed.  All
        // thumbnails are still hidden at this point.
        self.applications = Some(applications);
        unsafe {
            ShowWindow(self.hwnd, SW_SHOW);
        }
        self.paint_owner_synchronously();
        if let Some(applications) = self.applications.as_mut()
            && applications.set_visible(true).is_err()
        {
            self.close_session_resources();
            return Err(());
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
            let _ = current.set_visible(false);
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
        self.queue_desktop_snapshot_refresh(virtual_bounds());
    }

    fn queue_desktop_snapshot_refresh(&mut self, bounds: RECT) {
        self.reap_finished_snapshot_worker();
        if self.disposed
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

    /// Publish a completed worker capture on the UI thread. Captures which
    /// complete while the switcher is visible are rejected like the original
    /// BeginInvoke callback.
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

        let current_bounds = virtual_bounds();
        if !same_rect(current_bounds, completion.bounds) {
            self.queue_desktop_snapshot_refresh(current_bounds);
            return;
        }

        self.desktop_snapshot = Some(completion.snapshot);
        self.desktop_snapshot_bounds = completion.bounds;
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    /// Compatibility entry point for an in-process acceptance harness. The
    /// capture remains asynchronous and only publishes while idle.
    pub fn refresh_snapshot_while_idle(&mut self) {
        if self.disposed || self.applications.is_some() {
            return;
        }
        self.queue_desktop_snapshot_refresh_current();
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

    fn paint_owner_synchronously(&self) {
        let mut client = RECT::default();
        if unsafe { GetClientRect(self.hwnd, &mut client) } == 0 {
            return;
        }
        let dc = unsafe { windows_sys::Win32::Graphics::Gdi::GetDC(self.hwnd) };
        if !dc.is_null() {
            self.paint(dc, client);
            unsafe {
                windows_sys::Win32::Graphics::Gdi::ReleaseDC(self.hwnd, dc);
            }
        }
        unsafe {
            UpdateWindow(self.hwnd);
        }
    }

    fn set_owner_bounds(&mut self, bounds: RECT) {
        self.owner_bounds = bounds;
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

fn virtual_bounds() -> RECT {
    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    RECT {
        left,
        top,
        right: left.saturating_add(width),
        bottom: top.saturating_add(height),
    }
}

fn same_rect(left: RECT, right: RECT) -> bool {
    left.left == right.left
        && left.top == right.top
        && left.right == right.right
        && left.bottom == right.bottom
}
