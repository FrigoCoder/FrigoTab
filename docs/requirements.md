# FrigoTab requirements and acceptance behaviors

This is the observable behavior inventory for acceptance-test-driven development of the current C# application. It describes what a user can see or do and what native Windows behavior must remain intact; implementation details are not requirements.

## Test policy

The automated gate contains 28 green plain C# MSTest acceptance tests in three timestamped classes. They use real application objects and real Windows resources:

- 15 tests exercise the actual switcher/session forms, candidate HWNDs, layout, DWM previews, pointer routing, activation, and cleanup.
- 7 tests exercise real window classification, stale HWND/layout handling, shell desktop capture, DWM visibility, and native resource lifetime.
- 6 tests launch the actual executable and verify startup, single-instance behavior, a real session HWND, pointer activation, and the first desktop frame.

Each test class and matching file starts with `TYYYYMMDDTHHMMSSZ_NNN_DescriptiveFamilyName`. The timestamp and family suffix are stable identifiers; methods use descriptive plain C# names.

The global hook deliberately ignores injected keyboard events. Consequently, deterministic tests cover the real objects and executable, while physical keyboard transitions remain in the manual Windows checklist.

## Current behaviors

| Area | Requirement | Evidence |
| --- | --- | --- |
| Startup | Start a form-less WinForms message loop, own a per-user single-instance guard, show a tray icon, avoid a taskbar button, and keep the switcher hidden until a gesture. | Launched-process and real mutex acceptance tests; manual startup check. |
| Tray exit | Exit closes an active session, disposes native resources, and ends the process. | Real session cleanup and process tests; manual tray check. |
| First Alt+Tab | A successful Alt+Tab opens the overlay and selects the first eligible candidate. An empty or failed opening leaves the native gesture usable. | Real session and executable acceptance tests; physical-hook check. |
| Sticky Alt release | Releasing Alt deliberately leaves the overlay open for an explicit choice. Immediate release and held-then-released Alt have the same behavior. | Real session acceptance test; physical-hook check. |
| Repeated navigation | Tab advances and wraps; Shift+Alt+Tab moves backward and wraps. The overlay remains available for further input. | Real session acceptance tests; physical-hook check. |
| Keyboard balance | A consumed Tab, digit, Escape, or F4 key-down consumes its matching key-up, including when the session closes. Unhandled events continue through the hook. | Real session acceptance tests; physical-hook check. |
| Number selection | D1..D9 and NumPad1..NumPad9 select the same one-based candidates. Invalid numbers do not change the selection. | Real session acceptance tests; physical keyboard/layout check. |
| Cancellation | Escape and Alt+F4 close only the overlay; they do not activate a target or terminate the tray process. | Real session acceptance tests; physical-hook check. |
| Pointer selection | Hover selects the tile under the pointer. Moving outside clears selection, and keyboard navigation can select again. | Real session acceptance tests; manual layered-window hit-test check. |
| Pointer activation | Clicking a tile activates exactly the target under the pointer and closes the overlay after a successful activation. | Real session and executable acceptance tests; manual focus check. |
| Foreground denial | Restoring and foregrounding a target is best effort. A denial leaves the overlay usable and does not join input queues merely because focus was denied. | Real activation acceptance tests; manual UIPI/elevation/focus check. |
| Candidate windows | Enumerate eligible visible top-level application windows and exclude invisible, disabled, cloaked, no-activate, and tool-window candidates. | Real window acceptance tests; manual shell/tool-window check. |
| Stale candidates | A window that disappears during layout or activation is skipped or fails recoverably without invalidating the rest of the session. | Real HWND/layout and session acceptance tests; manual race check. |
| Layout | Use restored placement to choose a minimized window's monitor; preserve negative origins, working-area margins, stable numbering, and source aspect ratios. | Real layout acceptance tests; manual mixed-DPI/portrait check. |
| Shell desktop backdrop | Render Explorer's desktop host (`Progman`, or the matching `WorkerW`) into an off-screen bitmap containing wallpaper and icons. Do not capture the screen or compose the background from application windows. | Real shell-backdrop acceptance tests; manual Explorer/restart/RDP check. |
| First frame | Prepare or reuse the shell snapshot before opening, paint the owner from it synchronously, and reveal DWM previews only after that paint. The first visible frame must not show application pixels. | Real process first-frame acceptance test; manual visual check. |
| Shell fallback | If the shell host or render is unavailable, use the last matching frame when possible or black, release partial resources, and keep opening usable. | Real shell-backdrop acceptance tests; manual protected/RDP check. |
| DWM previews | Configure destination/source geometry, keep each thumbnail hidden until the owner backdrop is visible, surface native failures, and unregister/hide deterministically. | Real DWM acceptance tests; manual DWM-disabled/protected-surface check. |
| Resource lifetime | Forms, thumbnails, icons, fonts, bitmaps, hook handles, tray resources, and mutexes are released on normal close, failed construction, and process exit. | Real cleanup/resource acceptance tests; manual repeated-session handle check. |
| Activation handoff | Restore minimized targets and attempt foreground activation without `AttachThreadInput`. The intentional deactivation during handoff is not treated as an external interruption. | Real activation acceptance tests; manual focus/taskbar check. |
| Interruption | Lock/unlock, desktop deactivation, tray exit, display/DPI change, and DWM composition changes reset or safely close the current session and input state. | Manual Windows matrix; selected real-session cleanup coverage. |

## Explicit caveats

- The backdrop is a static shell frame prepared while idle; applications can change behind the overlay without changing that session's wallpaper/icon image.
- Sticky Alt release is the default behavior. The initial candidate is not activated merely because Alt was released; choose with Tab/Shift+Tab, D1..D9, NumPad1..NumPad9, or the pointer, or cancel with Escape/Alt+F4.
- Protected, secure, unavailable, or capture-disabled surfaces may render black.
- A display/DPI topology change may close the session rather than rebuild every native tile in place.
- `SetForegroundWindow` can be denied by Windows focus/UIPI policy. The overlay remains admitted and usable when that happens.
- Physical hook behavior, unusual shell hosts, Explorer restarts, protected surfaces, and mixed monitor/DPI configurations still require manual validation.
