# FrigoTab architecture and test boundaries

FrigoTab is a small native Win32 application written in Rust around Win32 window enumeration, DWM thumbnails, and the Explorer desktop surface. The application has one UI message loop. The low-level keyboard hook does only bounded event handling and posts session work to that loop; enumeration, window construction, painting, and native preview setup do not run inside the hook callback.

## Current component map

| Component | Responsibility | Source |
| --- | --- | --- |
| Composition root | Starts the native Win32 loop, owns lifetime, and connects the tray, hook, and single-instance guard. | `src/main.rs`, `src/single_instance_guard.rs` |
| Tray | Creates the native notification-area icon, Exit command, and the runtime-only menus for Alt-release and backdrop behavior. | `src/sys_tray_icon.rs`, `src/main.rs` |
| Keyboard hook | Installs `WH_KEYBOARD_LL`, tracks physical modifier transitions, forwards bounded input events, and keeps the native callback safe. | `src/key_hook.rs`, `src/keyboard_modifier_state.rs`, `src/keyboard_input.rs` |
| Switcher controller | Opens, navigates, cancels, commits, and closes one session while keeping consumed keyboard gestures balanced. | `src/switcher_application.rs`, `src/key_handling.rs`, `src/switcher_state.rs`, `src/keyboard_suppression_state.rs`, `src/deferred_keyboard_dispatcher.rs`, `src/alt_tab_recovery_plan.rs` |
| Session view | Owns the actual overlay owner window, preview windows, pointer routing, selection rendering, the selected backdrop mode, shell backdrop, and activation calls. | `src/session_window.rs`, `src/frigo_window.rs`, `src/application_window.rs`, `src/application_windows.rs` |
| Window catalog | Enumerates and classifies eligible top-level application HWNDs. | `src/window_finder.rs`, `src/window_handle.rs`, `src/window_icon.rs` |
| Window/layout | Reads titles, icons, styles, placement, monitor geometry, and activation state; assigns stable tile rectangles. | `src/layout.rs`, `src/rect.rs`, `src/points.rs`, `src/screen_point.rs` |
| DWM preview | Registers thumbnails, applies destination geometry and visibility while the owner is hidden, and disposes handles on close. | `src/thumbnail.rs` |
| Layered overlay | Renders the selected tint, title, icon, and number into the owned layered preview windows. | `src/layer_updater.rs`, `src/gdi_plus.rs` |
| Shell backdrop | Captures Explorer's wallpaper-and-icons surface into a retained native bitmap for Full desktop mode; the session also supports direct image-only and black painting. | `src/shell_desktop_snapshot.rs`, `src/session_window.rs` |
| Observable state | Publishes selected and visible changes used by the session and controller. | `src/application_windows.rs`, `src/switcher_state.rs` |
| Acceptance boundary | Runs the same production HWNDs, shell/DWM calls, native resources, and executable used by the application. | `acceptance-tests/src/lib.rs`, `acceptance-tests/tests/*.rs` |

The application creates one preview window per selectable candidate. The backdrop is not composed from those application windows.

The owner HWND stores one stable context. Mutable application state is protected by checked `RefCell` borrows, while its cloneable `SessionPainter` owns only the independently shared backdrop state. This matters because Win32 may synchronously deliver `WM_PAINT` from inside another native call: the callback can repaint the real first frame without creating a second mutable reference to the application or session.

## Session and input flow

1. The hook observes a non-injected Alt+Tab transition and posts the bounded event to the UI loop.
2. The controller asks the session view to enumerate eligible windows and build the overlay.
3. The view lays out real candidate HWNDs and configures their DWM thumbnails while the owner remains hidden, allowing the compositor to prepare the redirected source surfaces off-screen.
4. The view shows and synchronously paints the owner from the selected backdrop (the retained shell snapshot in Full desktop mode), then shows the already-rendered title/number overlays. Tab/Shift+Tab, numbers, and pointer movement update the selected candidate.
5. In the default Sticky mode, releasing Alt does not commit the first candidate; the session remains available until an explicit number/pointer activation or Escape/Alt+F4 cancellation. Tray-selected Tap (classic) commits the current selection on Alt release.
6. Activation restores a minimized target and attempts `SetForegroundWindow` after the historical input nudge. The controller then closes and disposes the session.

The handoff intentionally does not use `AttachThreadInput`; joining input queues can corrupt focus and key-state isolation. Windows may still deny foreground activation, in which case the overlay remains usable.

## Shell desktop snapshot

In the default `FullDesktop` mode, `ShellDesktopSnapshot` locates Explorer's desktop host (`Progman`, or the `WorkerW` containing `SHELLDLL_DefView`/`SysListView32`) and calls `PrintWindow` with `PW_RENDERFULLCONTENT` into an off-screen compatible bitmap. The resulting top-down DIB is retained while the application is idle and reused when a session opens. This avoids a screen capture and avoids searching/filtering/composing the background from application windows. `ImageOnly` calls `PaintDesktop` directly into the owner DC, so it paints the desktop pattern or wallpaper without desktop icons. `Black` fills the owner with `PatBlt(BLACKNESS)` and does not touch Explorer.

If the shell host or render is unavailable in `FullDesktop` mode, the most recent matching frame is used when possible; otherwise the owner paints black. The image-only and black modes do not require a shell snapshot. Partially created HDCs, HBITMAPs, and native wrappers are released on every failure path. A display/DPI topology change can close the current session and request a correctly sized replacement.

## DWM and resource lifetime

Each preview registers a DWM thumbnail against the hidden owner. Its destination, opacity, and visibility are set together so DWM can prepare the live source before the owner is exposed; owner visibility still prevents a partial session from appearing. Teardown hides the owner and unregisters each thumbnail with its native resources. When DWM is unavailable, the tile retains its icon/title fallback.

Windows, icons, fonts, layered DCs, bitmaps, thumbnails, hook handles, the tray icon, and the single-instance guard have explicit ownership and are disposed during normal close, failed construction, tray Exit, and process shutdown.

## Acceptance boundary

The automated gate has 41 tests in seven timestamped plain Rust integration-test modules. The tests use real HWNDs and native Windows resources, including DWM/GDI, the Explorer shell surface, the real tray popup, and the launched executable. They verify observable behavior such as Sticky and Tap release, navigation, selection, tray-only runtime settings, all backdrop modes, stale-window recovery, first-frame backdrop ordering, first-frame thumbnail readiness, DWM visibility, process lifetime, visual parity, and cleanup.

The global hook ignores injected events by design. Therefore physical Alt/Tab transitions, focus/UIPI restrictions, Explorer restart, protected surfaces, lock/unlock, mixed monitor/DPI topology, and long-running native handle behavior remain manual release checks. The automated results are necessary evidence, not a claim that every Windows desktop configuration is identical.
