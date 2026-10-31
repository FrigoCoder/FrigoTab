//! Low-rate background capture of the current fullscreen foreground window.

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows_sys::Win32::Graphics::Gdi::{
    COLORONCOLOR, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
    SRCCOPY, SetStretchBltMode, StretchBlt,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetForegroundWindow, GetWindowLongPtrW, GetWindowPlacement, GetWindowRect,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, SW_SHOWMAXIMIZED,
    WINDOWPLACEMENT, WS_CAPTION,
};

use super::capture_surface::CaptureSurface;
use super::fullscreen_preview::FullscreenPreview;
use super::thumbnail::DwmThumbnail;

const CAPTURE_INTERVAL: Duration = Duration::from_millis(500);
const MAX_CACHE_ENTRIES: usize = 16;
const MAX_PREVIEW_WIDTH: i32 = 640;
const MAX_PREVIEW_HEIGHT: i32 = 360;

/// Maintains a small cache while the application is idle. Capture runs on one
/// worker; readers only clone an `Arc` to the latest completed frame.
pub struct FullscreenPreviewCache {
    owner: isize,
    owner_pid: u32,
    entries: Arc<Mutex<VecDeque<FullscreenPreview>>>,
    stop_sender: Option<Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl FullscreenPreviewCache {
    /// Start observing the desktop independently of any switcher session.
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque window identity.
    pub fn new(owner: HWND) -> Self {
        let owner = owner as isize;
        let owner_pid = window_pid(owner as HWND);
        let entries = Arc::new(Mutex::new(VecDeque::new()));
        let worker_entries = Arc::clone(&entries);
        let (stop_sender, stop_receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("FrigoTab fullscreen preview".to_string())
            .spawn(move || capture_loop(owner, owner_pid, worker_entries, stop_receiver))
            .ok();

        Self {
            owner,
            owner_pid,
            entries,
            stop_sender: Some(stop_sender),
            worker,
        }
    }

    /// Return a cached fullscreen marker for this exact HWND/PID pair.
    ///
    /// The optional image is cloned by reference count, so this never waits
    /// for capture or copies the image pixels.
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque window identity.
    pub fn get(&self, hwnd: HWND) -> Option<FullscreenPreview> {
        let pid = window_pid(hwnd);
        if hwnd.is_null() || pid == 0 {
            return None;
        }
        self.entries
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .find(|preview| preview.hwnd == hwnd as isize && preview.pid == pid)
            .cloned()
    }

    /// Record the placeholder immediately when session setup has already
    /// identified a fullscreen source but the background worker has not yet
    /// observed it. This performs no capture.
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is an opaque window identity.
    pub(crate) fn mark_fullscreen(&self, hwnd: HWND) -> bool {
        if hwnd.is_null() || hwnd as isize == self.owner {
            return false;
        }
        let pid = window_pid(hwnd);
        if pid == 0 || pid == self.owner_pid {
            return false;
        }
        match fullscreen_bounds(self.owner as HWND, hwnd) {
            Ok(Some(_)) => {
                upsert(&self.entries, hwnd as isize, pid, None);
                true
            }
            Ok(None) => {
                remove(&self.entries, hwnd as isize, pid);
                false
            }
            Err(()) => false,
        }
    }

    /// Stop capture before the owner HWND is destroyed. Repeated calls are safe.
    pub(crate) fn stop(&mut self) {
        if let Some(sender) = self.stop_sender.take() {
            let _ = sender.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for FullscreenPreviewCache {
    fn drop(&mut self) {
        self.stop();
    }
}

fn capture_loop(
    owner: isize,
    owner_pid: u32,
    entries: Arc<Mutex<VecDeque<FullscreenPreview>>>,
    stop_receiver: Receiver<()>,
) {
    loop {
        match stop_receiver.try_recv() {
            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => return,
            Err(mpsc::TryRecvError::Empty) => {}
        }

        capture_foreground(owner, owner_pid, &entries);
        match stop_receiver.recv_timeout(CAPTURE_INTERVAL) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

fn capture_foreground(owner: isize, owner_pid: u32, entries: &Mutex<VecDeque<FullscreenPreview>>) {
    prune_closed(entries, owner, owner_pid);

    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() || hwnd as isize == owner {
        return;
    }
    let pid = window_pid(hwnd);
    if pid == 0 || pid == owner_pid {
        return;
    }
    let (capture_bounds, window_bounds) = match fullscreen_bounds(owner as HWND, hwnd) {
        Ok(Some(bounds)) => bounds,
        Ok(None) => {
            remove(entries, hwnd as isize, pid);
            return;
        }
        Err(()) => return,
    };

    // Publish the marker before attempting GDI. A capture failure still leaves
    // the session code a cheap black-tile fallback.
    upsert(entries, hwnd as isize, pid, None);
    if let Some(frame) = capture_frame(hwnd, capture_bounds, window_bounds) {
        upsert(entries, hwnd as isize, pid, Some(frame));
    }
}

/// Distinguish a valid windowed foreground source from a transient query
/// failure. Only the former invalidates a previously cached fullscreen frame.
fn fullscreen_bounds(owner: HWND, hwnd: HWND) -> Result<Option<(RECT, RECT)>, ()> {
    if unsafe { IsWindow(hwnd) } == 0
        || unsafe { IsWindowVisible(hwnd) } == 0
        || unsafe { IsIconic(hwnd) } != 0
    {
        return Err(());
    }
    let bounds = physical_window_bounds(hwnd).ok_or(())?;
    let mut placement = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    if unsafe { GetWindowPlacement(hwnd, &mut placement) } == 0 {
        return Err(());
    }
    // A normal captioned maximized application can cover the monitor when the
    // taskbar auto-hides. Keep it on live DWM; borderless fullscreen games do
    // not carry a caption even if they use the maximized show state.
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
    if placement.showCmd == SW_SHOWMAXIMIZED as u32 && style & WS_CAPTION != 0 {
        return Ok(None);
    }
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return Err(());
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return Err(());
    }
    if covers(bounds, info.rcMonitor) {
        return Ok(Some((bounds, bounds)));
    }
    if source_matches_monitor(owner, hwnd, bounds, info.rcMonitor) {
        return Ok(Some((info.rcMonitor, bounds)));
    }
    Ok(None)
}

fn capture_frame(hwnd: HWND, bounds: RECT, window_bounds: RECT) -> Option<(Arc<[u32]>, i32, i32)> {
    if unsafe { GetForegroundWindow() } != hwnd {
        return None;
    }
    let source_width = bounds.right.checked_sub(bounds.left)?;
    let source_height = bounds.bottom.checked_sub(bounds.top)?;
    if source_width <= 0 || source_height <= 0 {
        return None;
    }
    let scale = (MAX_PREVIEW_WIDTH as f64 / source_width as f64)
        .min(MAX_PREVIEW_HEIGHT as f64 / source_height as f64)
        .min(1.0);
    let width = (source_width as f64 * scale).round().max(1.0) as i32;
    let height = (source_height as f64 * scale).round().max(1.0) as i32;
    let pixel_count = (width as usize).checked_mul(height as usize)?;
    let surface = CaptureSurface::new(width, height)?;

    unsafe {
        if SetStretchBltMode(surface.memory_dc(), COLORONCOLOR) == 0
            || StretchBlt(
                surface.memory_dc(),
                0,
                0,
                width,
                height,
                surface.screen_dc(),
                bounds.left,
                bounds.top,
                source_width,
                source_height,
                SRCCOPY,
            ) == 0
        {
            return None;
        }
    }

    // Recheck foreground and geometry so a fast Alt+Tab cannot cache pixels
    // from whatever appeared over the game during the copy.
    if unsafe { GetForegroundWindow() } != hwnd
        || !physical_window_bounds(hwnd).is_some_and(|current| same_rect(window_bounds, current))
    {
        return None;
    }

    // A top-down 32-bit DIB is BGRA in memory. Mark it opaque for GDI+ and
    // copy it to the shared frame before releasing its native bitmap.
    let dib = unsafe { std::slice::from_raw_parts(surface.bits(), pixel_count) };
    if dib.iter().all(|pixel| pixel & 0x00ff_ffff == 0) {
        return None;
    }
    let pixels: Arc<[u32]> = dib
        .iter()
        .map(|pixel| pixel | 0xff00_0000)
        .collect::<Vec<_>>()
        .into();
    Some((pixels, width, height))
}

fn physical_window_bounds(hwnd: HWND) -> Option<RECT> {
    let mut bounds = RECT::default();
    let hresult = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            (&mut bounds as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
    };
    if hresult >= 0 && bounds.right > bounds.left && bounds.bottom > bounds.top {
        return Some(bounds);
    }
    if unsafe { GetWindowRect(hwnd, &mut bounds) } != 0
        && bounds.right > bounds.left
        && bounds.bottom > bounds.top
    {
        Some(bounds)
    } else {
        None
    }
}

fn source_matches_monitor(owner: HWND, hwnd: HWND, bounds: RECT, monitor: RECT) -> bool {
    let window_width = i64::from(bounds.right - bounds.left);
    let window_height = i64::from(bounds.bottom - bounds.top);
    let monitor_width = i64::from(monitor.right - monitor.left);
    let monitor_height = i64::from(monitor.bottom - monitor.top);
    if window_width <= 0
        || window_height <= 0
        || monitor_width <= 0
        || monitor_height <= 0
        || (bounds.left - monitor.left).abs() > 16
        || (bounds.top - monitor.top).abs() > 16
        || window_width * 4 < monitor_width * 3
        || window_height * 4 < monitor_height * 3
        || (window_width * monitor_height - window_height * monitor_width).abs() * 100
            > monitor_width * monitor_height * 2
    {
        return false;
    }
    DwmThumbnail::register(owner, hwnd)
        .and_then(|thumbnail| thumbnail.source_size())
        .is_ok_and(|size| {
            i64::from(size.cx) == monitor_width && i64::from(size.cy) == monitor_height
        })
}

fn covers(bounds: RECT, monitor: RECT) -> bool {
    bounds.left <= monitor.left
        && bounds.top <= monitor.top
        && bounds.right >= monitor.right
        && bounds.bottom >= monitor.bottom
}

fn upsert(
    entries: &Mutex<VecDeque<FullscreenPreview>>,
    hwnd: isize,
    pid: u32,
    frame: Option<(Arc<[u32]>, i32, i32)>,
) {
    let mut entries = entries.lock().unwrap_or_else(|error| error.into_inner());
    let mut preview = entries
        .iter()
        .position(|item| item.hwnd == hwnd && item.pid == pid)
        .and_then(|index| entries.remove(index))
        .unwrap_or(FullscreenPreview {
            hwnd,
            pid,
            image: None,
            width: 0,
            height: 0,
        });
    if let Some((pixels, width, height)) = frame {
        preview.image = Some(pixels);
        preview.width = width;
        preview.height = height;
    }
    entries.push_back(preview);
    while entries.len() > MAX_CACHE_ENTRIES {
        entries.pop_front();
    }
}

fn remove(entries: &Mutex<VecDeque<FullscreenPreview>>, hwnd: isize, pid: u32) {
    entries
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|preview| preview.hwnd != hwnd || preview.pid != pid);
}

fn prune_closed(entries: &Mutex<VecDeque<FullscreenPreview>>, owner: isize, owner_pid: u32) {
    let mut entries = entries.lock().unwrap_or_else(|error| error.into_inner());
    entries.retain(|preview| {
        let hwnd = preview.hwnd as HWND;
        hwnd as isize != owner
            && preview.pid != owner_pid
            && unsafe { IsWindow(hwnd) } != 0
            && window_pid(hwnd) == preview.pid
    });
}

fn window_pid(hwnd: HWND) -> u32 {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
    }
    pid
}

fn same_rect(left: RECT, right: RECT) -> bool {
    left.left == right.left
        && left.top == right.top
        && left.right == right.right
        && left.bottom == right.bottom
}
