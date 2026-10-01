//! Native session-window adapter corresponding to the original session form.
//!
//! The controller remains unaware of HWNDs and DWM.  This type owns the one
//! full-desktop owner window, the retained Explorer frame, and the current
//! `ApplicationWindows` collection. The order in `try_open` is intentional:
//! prepare DWM previews behind the hidden owner, show and synchronously complete
//! the owner's paint cycle, reveal the layered overlays, then request foreground
//! activation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, HDC, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
    UpdateWindow,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ClipCursor, GetClientRect, GetCursorPos, GetForegroundWindow, GetWindowRect, HWND_TOPMOST,
    IsWindowVisible, KillTimer, PostMessageW, SC_CLOSE, SW_HIDE, SW_SHOW, SWP_NOACTIVATE,
    SWP_NOOWNERZORDER, SetTimer, SetWindowPos, ShowWindow, WM_APP, WM_SYSCOMMAND,
};

use crate::desktop::shell_desktop_snapshot::ShellDesktopSnapshot;
use crate::geometry::display_mode::current_display_geometry_is_persisted;
use crate::geometry::rect::virtual_screen_bounds;
use crate::geometry::screen_point::ScreenPoint;
use crate::window::{ApplicationWindows, WindowFinder, WindowHandle};

pub use super::background_mode::BackgroundMode;
pub use super::close_button_mode::CloseButtonMode;
pub use super::session_painter::SessionPainter;
use super::snapshot_completion::SnapshotCompletion;
use super::switcher_application::SwitcherSessionPort;
use super::window_dc::WindowDc;

pub const WM_DESKTOP_SNAPSHOT_READY: u32 = WM_APP + 3;
pub const APPLICATION_REFRESH_TIMER_ID: usize = 2;
pub const DISPLAY_RELAYOUT_TIMER_ID: usize = 3;
const APPLICATION_REFRESH_INTERVAL_MS: u32 = 100;
const DISPLAY_RELAYOUT_INTERVAL_MS: u32 = 250;
const DISPLAY_STABLE_TICKS: u8 = 2;
const DISPLAY_MAX_WAIT_TICKS: u8 = 20;
const APPLICATION_REFRESH_CONFIRMATION_TICKS: u8 = 2;
const EMPTY_ENUMERATION_CONFIRMATION_TICKS: u8 = 3;

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
    close_button_mode: CloseButtonMode,
    pending_closes: Vec<HWND>,
    refresh_evidence_ticks: u8,
    activating_selection: bool,
    display_relayout_pending: bool,
    display_relayout_bounds: RECT,
    display_relayout_stable_ticks: u8,
    display_relayout_elapsed_ticks: u8,
    display_rebuild_requested: bool,
    disposed: bool,
}

impl SessionWindow {
    /// Create the hidden owner and capture the shell frame before the hook is
    /// installed when the foreground/display state is a normal desktop. The
    /// caller creates/registers the HWND so its WndProc can be installed in
    /// the same way as the original `FrigoForm`.
    pub fn new(hwnd: HWND) -> Self {
        let bounds = virtual_screen_bounds();
        let snapshot = (current_display_geometry_is_persisted() != Some(false)
            && !window_covers_monitor(unsafe { GetForegroundWindow() }))
        .then(|| ShellDesktopSnapshot::capture(bounds))
        .filter(ShellDesktopSnapshot::is_available);
        let snapshot_available = snapshot.is_some();
        let painter = SessionPainter::new(
            snapshot,
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
            close_button_mode: CloseButtonMode::default(),
            pending_closes: Vec::new(),
            refresh_evidence_ticks: 0,
            activating_selection: false,
            display_relayout_pending: false,
            display_relayout_bounds: RECT::default(),
            display_relayout_stable_ticks: 0,
            display_relayout_elapsed_ticks: 0,
            display_rebuild_requested: false,
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

    pub fn close_button_mode(&self) -> CloseButtonMode {
        self.close_button_mode
    }

    pub fn set_close_button_mode(&mut self, mode: CloseButtonMode) {
        if self.close_button_mode == mode || self.disposed {
            return;
        }
        self.close_button_mode = mode;
        if let Some(applications) = self.applications.as_mut() {
            applications.set_close_button_mode(mode);
        }
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
            // Refresh while idle even when a matching retained frame exists:
            // ImageOnly/Black may have been selected long enough for the
            // wallpaper or desktop icons to change.
            if self.applications.is_none() {
                self.queue_desktop_snapshot_refresh_current();
            }
        }
        self.invalidate_background();
    }

    pub fn applications(&self) -> Option<&ApplicationWindows> {
        self.applications.as_ref()
    }

    /// True while foreground activation is being handed to the selected
    /// application and until the session cleanup which follows a successful
    /// handoff. WM_ACTIVATEAPP(FALSE) during this small window is expected and
    /// must not interrupt the session.
    pub fn activating_selection(&self) -> bool {
        self.activating_selection
    }

    /// True while display/compositor notifications are being coalesced. A
    /// fullscreen application's deactivation can queue a stale
    /// WM_ACTIVATEAPP(FALSE) during this interval even though the owner is
    /// already foreground.
    pub fn display_relayout_pending(&self) -> bool {
        self.display_relayout_pending
    }

    /// Coalesce the burst of display, DPI, and compositor messages produced
    /// when an exclusive fullscreen application yields the display. Reusing
    /// one timer ID restarts the interval for each message in the burst.
    pub fn schedule_display_relayout(&mut self, rebuild_previews: bool) -> bool {
        if self.disposed {
            return false;
        }
        self.display_relayout_bounds = virtual_screen_bounds();
        self.display_relayout_stable_ticks = 0;
        self.display_relayout_elapsed_ticks = 0;
        self.display_rebuild_requested |= rebuild_previews;
        self.display_relayout_pending = unsafe {
            SetTimer(
                self.hwnd,
                DISPLAY_RELAYOUT_TIMER_ID,
                DISPLAY_RELAYOUT_INTERVAL_MS,
                None,
            ) != 0
        };
        self.display_relayout_pending
    }

    /// A display mode is settled only after two equal geometry samples and a
    /// read-only current-versus-persisted mode check. The latter prevents a
    /// long-running exclusive mode from becoming the new desktop baseline.
    pub fn display_relayout_ready(&mut self) -> bool {
        if self.disposed || !self.display_relayout_pending {
            return false;
        }
        self.display_relayout_elapsed_ticks = self.display_relayout_elapsed_ticks.saturating_add(1);
        let timed_out = self.display_relayout_elapsed_ticks >= DISPLAY_MAX_WAIT_TICKS;
        let current = virtual_screen_bounds();
        if current.right <= current.left || current.bottom <= current.top {
            self.display_relayout_stable_ticks = 0;
            return timed_out;
        }
        if !same_rect(current, self.display_relayout_bounds) {
            self.display_relayout_bounds = current;
            self.display_relayout_stable_ticks = 0;
            return timed_out;
        }
        if current_display_geometry_is_persisted() == Some(false) {
            self.display_relayout_stable_ticks = 0;
            return timed_out;
        }
        self.display_relayout_stable_ticks = self.display_relayout_stable_ticks.saturating_add(1);
        timed_out || self.display_relayout_stable_ticks >= DISPLAY_STABLE_TICKS
    }

    pub fn refresh_snapshot_after_display_change(&mut self) {
        let timed_out = self.display_relayout_elapsed_ticks >= DISPLAY_MAX_WAIT_TICKS;
        self.stop_display_relayout();
        if timed_out && current_display_geometry_is_persisted() == Some(false) {
            return;
        }
        self.queue_desktop_snapshot_refresh_current();
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

        let previous_foreground = unsafe { GetForegroundWindow() };
        let fullscreen_source = window_covers_monitor(previous_foreground);
        self.close_session_resources();
        let finder = WindowFinder::new();
        if finder.windows.is_empty() {
            return Ok(0);
        }

        let current_bounds = virtual_screen_bounds();
        if current_bounds.right <= current_bounds.left
            || current_bounds.bottom <= current_bounds.top
        {
            return Ok(0);
        }
        let temporary_mode = current_display_geometry_is_persisted() == Some(false);
        // A retained Explorer frame is one indivisible image/geometry pair.
        // Keep that geometry for the whole visible session instead of sizing
        // it to a game's temporary mode and painting black or a cropped frame.
        let bounds = self
            .painter
            .snapshot_bounds()
            .filter(|_| self.background_mode() == BackgroundMode::FullDesktop)
            .unwrap_or(current_bounds);
        if !self.set_owner_bounds(bounds) {
            return Err(());
        }

        if self.background_mode() == BackgroundMode::FullDesktop
            && !self.painter.has_snapshot_for(bounds)
            && !fullscreen_source
        {
            self.queue_desktop_snapshot_refresh_current();
        }

        let owner = WindowHandle::new(self.hwnd);
        let applications =
            ApplicationWindows::with_close_button_mode(owner, &finder, self.close_button_mode);
        if applications.is_empty() {
            return Ok(0);
        }

        // Publish only after every candidate has been constructed. DWM has
        // prepared the thumbnails behind this still-hidden owner.
        self.applications = Some(applications);
        if !self.start_application_refresh() {
            self.close_session_resources();
            return Err(());
        }
        unsafe {
            ShowWindow(self.hwnd, SW_SHOW);
        }
        self.paint_owner_synchronously();
        if let Some(applications) = self.applications.as_mut() {
            set_pointer_hover(applications);
            applications.set_visible(true);
        }
        // Foreground activation is deliberately last; a SetForegroundWindow
        // deactivation is the expected selection handoff, not interruption.
        if unsafe { GetForegroundWindow() } != self.hwnd {
            let _ = owner.set_foreground();
        }
        release_cursor_clip();
        // This also supplies a short activation grace. Legacy exclusive games
        // can emit a stale owner deactivation before their display mode has
        // finished yielding; the settled timer gets one chance to restore the
        // owner rather than immediately tearing down the session.
        if temporary_mode || fullscreen_source {
            self.schedule_display_relayout(temporary_mode || !same_rect(bounds, current_bounds));
        }
        Ok(self
            .applications
            .as_ref()
            .map_or(0, ApplicationWindows::count))
    }

    fn close_session_resources(&mut self) {
        // Clear confinement left by the application we are leaving when the
        // session is cancelled/interrupted. A successful selection releases
        // the old clip before activation and then preserves any new clip the
        // selected application establishes for itself.
        let selected_application_was_activated = self.activating_selection;
        self.activating_selection = false;
        self.stop_display_relayout();
        self.stop_application_refresh();
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
        if applications.is_some() && !selected_application_was_activated {
            release_cursor_clip();
        }
        if applications.is_some() && !self.disposed {
            self.queue_desktop_snapshot_refresh_current();
        }
    }

    /// Queue the same idle Explorer refresh performed by SessionForm after a
    /// session closes. A read-only display-mode check protects the retained
    /// normal desktop from a game's temporary exclusive geometry while still
    /// allowing a persisted monitor/resolution change to replace it.
    pub fn queue_desktop_snapshot_refresh_current(&mut self) {
        if current_display_geometry_is_persisted() == Some(false) {
            // Keep a bounded idle retry alive in case the game restores its
            // normal mode without delivering another display notification.
            if self.applications.is_none() {
                self.schedule_display_relayout(false);
            }
            return;
        }
        let bounds = virtual_screen_bounds();
        self.queue_desktop_snapshot_refresh(bounds);
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
                self.queue_desktop_snapshot_refresh_current();
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

    fn set_owner_bounds(&mut self, bounds: RECT) -> bool {
        let placed = unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                bounds.left,
                bounds.top,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            ) != 0
        };
        if placed {
            self.owner_bounds = bounds;
            self.painter.set_owner_bounds(bounds);
        }
        placed
    }

    /// Rebuild the preview graph once a display/DPI notification burst has
    /// settled. The owner remains visible throughout so an exclusive source
    /// cannot repeatedly reacquire the display between notifications.
    fn relayout_session(&mut self) -> Result<usize, ()> {
        let rebuild_previews = self.display_rebuild_requested;
        let timed_out = self.display_relayout_elapsed_ticks >= DISPLAY_MAX_WAIT_TICKS;
        self.stop_display_relayout();
        if self.disposed {
            return Err(());
        }
        let Some(current) = self.applications.as_ref() else {
            return Err(());
        };
        let current_count = current.count();
        let current_bounds = virtual_screen_bounds();
        if current_bounds.right <= current_bounds.left
            || current_bounds.bottom <= current_bounds.top
            || current_display_geometry_is_persisted() == Some(false)
        {
            if !timed_out {
                self.schedule_display_relayout(true);
            }
            return Ok(current_count);
        }
        let bounds = self
            .painter
            .snapshot_bounds()
            .filter(|_| self.background_mode() == BackgroundMode::FullDesktop)
            .unwrap_or(current_bounds);
        if same_rect(bounds, self.owner_bounds) && !rebuild_previews {
            return Ok(current_count);
        }

        let selected_index = current.selected_index();
        let selected_source = selected_index.and_then(|index| {
            current
                .windows()
                .get(index)
                .map(|window| window.application())
        });
        let mut finder = WindowFinder::new();
        if finder.windows.is_empty() {
            // Enumeration can be momentarily empty while a legacy display
            // driver is completing its mode change. Keep the still-usable
            // graph and try again after the next native notification/tick.
            self.schedule_display_relayout(true);
            return Ok(current_count);
        }
        if let (Some(index), Some(source)) = (selected_index, selected_source)
            && index < finder.windows.len()
            && let Some(replacement_index) = finder
                .windows
                .iter()
                .position(|window| window.raw() == source)
        {
            finder.windows.swap(index, replacement_index);
        }

        let previous_bounds = self.owner_bounds;
        if !self.set_owner_bounds(bounds) {
            if !timed_out {
                self.schedule_display_relayout(true);
            }
            return Ok(current_count);
        }
        self.paint_owner_synchronously();

        let owner = WindowHandle::new(self.hwnd);
        let mut replacement =
            ApplicationWindows::prepared_hidden(owner, &finder, self.close_button_mode);
        if replacement.is_empty() {
            let _ = self.set_owner_bounds(previous_bounds);
            self.paint_owner_synchronously();
            self.schedule_display_relayout(true);
            return Ok(current_count);
        }
        let selected = selected_index.map(|index| index.min(replacement.count() - 1));
        if replacement.select_by_index(selected).is_err() {
            let _ = self.set_owner_bounds(previous_bounds);
            self.paint_owner_synchronously();
            self.schedule_display_relayout(true);
            return Ok(current_count);
        }
        set_pointer_hover(&mut replacement);

        // DWM can be unavailable during a compositor transition. The layered
        // title/icon fallback is still a usable preview, so thumbnail publish
        // and flush are deliberately best effort here.
        let _ = replacement.set_thumbnails_visible(true);
        replacement.set_visible(true);
        let _ = unsafe { DwmFlush() };
        if let Some(current) = self.applications.as_mut() {
            current.set_visible(false);
            let _ = current.set_thumbnails_visible(false);
        }
        let count = replacement.count();
        let mut previous = self.applications.replace(replacement);
        if let Some(previous) = previous.as_mut() {
            previous.dispose();
        }
        if unsafe { GetForegroundWindow() } != self.hwnd {
            let _ = owner.set_foreground();
        }
        release_cursor_clip();
        Ok(count)
    }

    fn refresh_application_session(&mut self) -> Result<Option<usize>, ()> {
        if self.disposed || self.applications.is_none() {
            self.stop_application_refresh();
            return Err(());
        }
        // Some games confine the cursor again from their input/render loop
        // after losing foreground. Keep clearing that process-global clip for
        // as long as FrigoTab owns the visible interaction surface.
        release_cursor_clip();

        let source_is_alive = |source: HWND| WindowHandle::new(source).is_valid();
        let requested_close_completed = self
            .pending_closes
            .iter()
            .any(|source| !source_is_alive(*source) || unsafe { IsWindowVisible(*source) == 0 });
        let source_disappeared = self.applications.as_ref().is_some_and(|applications| {
            applications
                .windows()
                .iter()
                .any(|window| !source_is_alive(window.application()))
        });
        if !requested_close_completed && !source_disappeared {
            self.refresh_evidence_ticks = 0;
            return Ok(None);
        }
        self.refresh_evidence_ticks = self.refresh_evidence_ticks.saturating_add(1);
        if self.refresh_evidence_ticks < APPLICATION_REFRESH_CONFIRMATION_TICKS {
            return Ok(None);
        }

        let finder = WindowFinder::new();
        let source_is_switchable =
            |source: HWND| finder.windows.contains(&WindowHandle::new(source));

        if finder.windows.is_empty() {
            let previous_candidate_is_alive =
                self.applications.as_ref().is_some_and(|applications| {
                    applications.windows().iter().any(|window| {
                        let source = window.application();
                        source_is_alive(source)
                            && !(self.pending_closes.contains(&source)
                                && unsafe { IsWindowVisible(source) == 0 })
                    })
                });
            if previous_candidate_is_alive {
                return Ok(None);
            }
            if self.refresh_evidence_ticks < EMPTY_ENUMERATION_CONFIRMATION_TICKS {
                return Ok(None);
            }
            self.refresh_evidence_ticks = 0;
            return Ok(Some(0));
        }
        self.refresh_evidence_ticks = 0;

        let owner = WindowHandle::new(self.hwnd);
        let mut replacement =
            ApplicationWindows::prepared_hidden(owner, &finder, self.close_button_mode);
        if replacement.is_empty() {
            return Err(());
        }
        set_pointer_hover(&mut replacement);

        // Reveal the fully prepared replacement while the previous graph is
        // still covering the owner. Only after DWM has composed that graph do
        // we hide and release the stale overlays and thumbnail registrations.
        // A source without a usable redirection surface still has a complete
        // icon/title fallback. One failed DWM update must not hide later
        // previews or tear down the live session.
        let _ = replacement.set_thumbnails_visible(true);
        replacement.set_visible(true);
        let _ = unsafe { DwmFlush() };
        if let Some(current) = self.applications.as_mut() {
            current.set_visible(false);
            let _ = current.set_thumbnails_visible(false);
        }
        let count = replacement.count();
        let mut previous = self.applications.replace(replacement);
        if let Some(previous) = previous.as_mut() {
            previous.dispose();
        }
        // Closing the foreground source can make Windows choose another
        // foreground window. Reassert the already-visible owner with the
        // original input-context nudge; this never joins application threads.
        if unsafe { GetForegroundWindow() } != self.hwnd {
            let _ = owner.set_foreground();
        }
        self.pending_closes
            .retain(|source| source_is_switchable(*source));
        Ok(Some(count))
    }

    fn track_pending_close(&mut self, target: HWND) -> bool {
        if self.pending_closes.contains(&target) {
            return true;
        }
        self.pending_closes.push(target);
        if !self.start_application_refresh() {
            self.pending_closes.retain(|source| *source != target);
            return false;
        }
        true
    }

    fn start_application_refresh(&self) -> bool {
        unsafe {
            SetTimer(
                self.hwnd,
                APPLICATION_REFRESH_TIMER_ID,
                APPLICATION_REFRESH_INTERVAL_MS,
                None,
            ) != 0
        }
    }

    fn stop_application_refresh(&mut self) {
        unsafe {
            KillTimer(self.hwnd, APPLICATION_REFRESH_TIMER_ID);
        }
        self.pending_closes.clear();
        self.refresh_evidence_ticks = 0;
    }

    fn stop_display_relayout(&mut self) {
        unsafe {
            KillTimer(self.hwnd, DISPLAY_RELAYOUT_TIMER_ID);
        }
        self.display_relayout_pending = false;
        self.display_relayout_bounds = RECT::default();
        self.display_relayout_stable_ticks = 0;
        self.display_relayout_elapsed_ticks = 0;
        self.display_rebuild_requested = false;
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

    fn set_hovered(&mut self, index: Option<usize>) -> Result<(), ()> {
        let Some(applications) = self.applications.as_mut() else {
            return Err(());
        };
        applications.set_hovered_index(index).map_err(|_| ())
    }

    fn set_pointer_index(&mut self, index: Option<usize>) -> Result<(), ()> {
        let Some(applications) = self.applications.as_mut() else {
            return Err(());
        };
        applications.set_pointer_index(index).map_err(|_| ())
    }

    fn try_close_at(&mut self, point: ScreenPoint) -> Result<bool, ()> {
        let Some(target) = self
            .applications
            .as_ref()
            .and_then(|applications| applications.close_target_at(point.x, point.y))
        else {
            if self.applications.is_none() {
                return Err(());
            }
            return Ok(false);
        };
        if self.pending_closes.contains(&target) {
            return Ok(true);
        }
        let posted = unsafe { PostMessageW(target, WM_SYSCOMMAND, SC_CLOSE as usize, 0) } != 0;
        let should_refresh = posted
            || !WindowHandle::new(target).is_valid()
            || unsafe { IsWindowVisible(target) == 0 };
        if should_refresh && !self.track_pending_close(target) {
            return Err(());
        }
        Ok(true)
    }

    fn refresh_closed_applications(&mut self) -> Result<Option<usize>, ()> {
        self.refresh_application_session()
    }

    fn try_activate_selected(&mut self) -> Result<bool, ()> {
        let Some(applications) = self.applications.as_ref() else {
            return Ok(false);
        };
        // Release the previous foreground application's confinement before
        // activating the selected target. Cleanup must not subsequently clear
        // a new clip legitimately established by that target.
        release_cursor_clip();
        self.activating_selection = true;
        let result = applications.try_activate_selected();
        if !result {
            self.activating_selection = false;
        }
        Ok(result)
    }

    fn close(&mut self) {
        if !self.disposed {
            self.close_session_resources();
        }
    }

    fn relayout(&mut self) -> Result<usize, ()> {
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

fn set_pointer_hover(applications: &mut ApplicationWindows) {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return;
    }
    let index = applications.hit_test(point.x, point.y);
    let _ = applications.set_hovered_index(index);
}

fn release_cursor_clip() {
    unsafe {
        ClipCursor(std::ptr::null());
    }
}

fn window_covers_monitor(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let mut window = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut window) } == 0 {
        return false;
    }
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return false;
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    (unsafe { GetMonitorInfoW(monitor, &mut info) } != 0)
        && window.left <= info.rcMonitor.left
        && window.top <= info.rcMonitor.top
        && window.right >= info.rcMonitor.right
        && window.bottom >= info.rcMonitor.bottom
}
