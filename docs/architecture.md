# FrigoTab architecture and test boundaries

FrigoTab is a small WinForms shell around Win32 window enumeration, DWM thumbnails, and the Explorer desktop surface. The application has one UI message loop. The low-level keyboard hook does only bounded event handling and posts session work to that loop; enumeration, form construction, painting, and native preview setup do not run inside the hook callback.

## Current component map

| Component | Responsibility | Source |
| --- | --- | --- |
| Composition root | Starts the form-less WinForms loop, owns lifetime, and connects the tray, hook, and single-instance guard. | `FrigoTab/Program.cs`, `FrigoTab/SingleInstanceGuard.cs` |
| Tray | Creates the `NotifyIcon` and Exit command. | `FrigoTab/SysTrayIcon.cs` |
| Keyboard hook | Installs `WH_KEYBOARD_LL`, tracks physical modifier transitions, forwards bounded input events, and keeps the native callback exception-safe. | `FrigoTab/KeyHook.cs` |
| Switcher controller | Opens, navigates, cancels, commits, and closes one session while keeping consumed keyboard gestures balanced. | `FrigoTab.Core/SwitcherApplication.cs` |
| Session view | Owns the actual overlay form, tile forms, pointer routing, selection rendering, shell backdrop, and activation calls. | `FrigoTab/SessionForm.cs`, `FrigoTab/ApplicationWindow.cs` |
| Window catalog | Enumerates and classifies eligible top-level application HWNDs. | `FrigoTab/WindowFinder.cs` |
| Window/layout | Reads titles, icons, styles, placement, monitor geometry, and activation state; assigns stable tile rectangles. | `FrigoTab/WindowHandle.cs`, `FrigoTab/Layout.cs`, `FrigoTab/GridLayout.cs` |
| DWM preview | Registers hidden thumbnails, applies source/destination geometry, reveals them after the owner backdrop is painted, and disposes handles on close. | `FrigoTab/Thumbnail.cs`, `FrigoTab/DwmThumbnailApi.cs` |
| Shell backdrop | Captures Explorer's wallpaper-and-icons surface into a retained native bitmap and paints it behind the previews. | `FrigoTab/ShellDesktopSnapshot.cs` |
| Observable state | Publishes selected/visible changes used by the view. | `FrigoTab/Property.cs`, `FrigoTab/ApplicationWindows.cs` |
| Acceptance boundary | Runs the same production forms, HWNDs, shell/DWM calls, and executable used by the application. | `FrigoTab.AcceptanceTests/*.cs` |

The application creates one `ApplicationWindow` form per selectable candidate. The backdrop is not composed from those application windows.

## Session and input flow

1. The hook observes a non-injected Alt+Tab transition and posts the bounded event to the UI loop.
2. The controller asks the session view to enumerate eligible windows and build the overlay.
3. The view lays out real candidate HWNDs, paints the retained shell snapshot, and keeps DWM thumbnails hidden while the first owner frame is being shown.
4. Once that frame is painted, previews and tile overlays are revealed. Tab/Shift+Tab, numbers, and pointer movement update the selected candidate.
5. Releasing Alt does not commit the first candidate. The session remains sticky until an explicit number/pointer activation or Escape/Alt+F4 cancellation.
6. Activation restores a minimized target and attempts `SetForegroundWindow` after the historical input nudge. The controller then closes and disposes the session.

The handoff intentionally does not use `AttachThreadInput`; joining input queues can corrupt focus and key-state isolation. Windows may still deny foreground activation, in which case the overlay remains usable.

## Shell desktop snapshot

`ShellDesktopSnapshot` locates Explorer's desktop host (`Progman`, or the `WorkerW` containing `SHELLDLL_DefView`/`SysListView32`) and calls `PrintWindow` with `PW_RENDERFULLCONTENT` into an off-screen compatible bitmap. The resulting top-down DIB is retained while the application is idle and reused when a session opens. This avoids a screen capture and avoids searching/filtering/composing the background from application windows.

If the shell host or render is unavailable, the most recent matching frame is used when possible; otherwise the owner paints black. Partially created HDCs, HBITMAPs, and managed wrappers are released on every failure path. A display/DPI topology change can close the current session and request a correctly sized replacement.

## DWM and resource lifetime

Each preview registers a DWM thumbnail against the visible owner. Destination and source rectangles are set explicitly, and the thumbnail is not made visible until the owner has painted the shell backdrop. Teardown hides and unregisters the thumbnail and releases its native resources. When DWM is unavailable, the tile retains its icon/title fallback.

Forms, icons, fonts, layered DCs, bitmaps, thumbnails, hook handles, the tray icon, and the single-instance guard have explicit ownership and are disposed during normal close, failed construction, tray Exit, and process shutdown.

## Acceptance boundary

The automated gate has 28 tests in three timestamped plain C# MSTest classes. The tests use real forms and HWNDs, real DWM/GDI and shell objects, and a launched executable. They verify observable behavior such as sticky release, navigation, selection, stale-window recovery, first-frame backdrop ordering, DWM visibility, process lifetime, and cleanup.

The global hook ignores injected events by design. Therefore physical Alt/Tab transitions, focus/UIPI restrictions, Explorer restart, protected surfaces, lock/unlock, mixed-DPI monitor topology, and long-running native handle behavior remain manual release checks. The automated results are necessary evidence, not a claim that every Windows desktop configuration is identical.
