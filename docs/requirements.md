# FrigoTab requirements and acceptance behaviors

This is the observable behavior inventory of the current native Rust application. It describes what a user can see or do and what native Windows behavior must remain intact; implementation details are not requirements.

## Test policy

The automated gate contains 41 green plain Rust integration acceptance tests. They use real application objects and real Windows resources:

- 15 tests exercise the actual switcher/session windows, candidate HWNDs, layout, DWM previews, pointer routing, activation, and cleanup.
- 7 tests exercise real window classification, stale HWND/layout handling, shell desktop capture, DWM visibility, and native resource lifetime.
- 6 tests launch the actual executable and verify startup, single-instance behavior, a real session HWND, pointer activation, and the first desktop frame.
- 3 tests inspect the real layered preview-window topology and rendered visual details.
- 5 tests exercise Sticky and Tap (classic) Alt-release behavior on a real session.
- 4 tests launch the actual executable, open its real tray popup, and verify runtime background-mode selection and painting.
- 1 test launches the actual executable and verifies that DWM publishes a live preview in the first composed owner frame.

The seven timestamped integration-test files are:

- `acceptance-tests/tests/t20260914t211700z_001_real_switcher_acceptance_tests.rs`
- `acceptance-tests/tests/t20260914t212400z_002_real_window_and_backdrop_acceptance_tests.rs`
- `acceptance-tests/tests/t20260914t212400z_003_real_process_acceptance_tests.rs`
- `acceptance-tests/tests/t20260914t223000z_004_real_visual_parity_acceptance_tests.rs`
- `acceptance-tests/tests/t20260915t175100z_005_alt_tab_behavior_acceptance_tests.rs`
- `acceptance-tests/tests/t20260915t175100z_006_tray_and_background_acceptance_tests.rs`
- `acceptance-tests/tests/t20260915t212300z_007_thumbnail_reveal_performance_acceptance_tests.rs`

The timestamp and family suffix are stable identifiers; test functions use descriptive plain Rust names.

The global hook deliberately ignores injected keyboard events. Consequently, deterministic tests cover the real objects and executable, while physical keyboard transitions remain in the manual Windows checklist.

## Current behaviors

| Area | Requirement | Evidence |
| --- | --- | --- |
| Startup | Start a native Win32 message loop, own a per-user single-instance guard, show a tray icon, avoid a taskbar button, and keep the switcher hidden until a gesture. | Launched-process and real mutex acceptance tests; manual startup check. |
| Tray exit | Exit closes an active session, disposes native resources, and ends the process. | Real session cleanup and process tests; manual tray check. |
| Tray settings | The tray menu is the only settings surface. It selects Sticky (the default) or Tap (classic) Alt-release behavior, and Full desktop (the default), Background image only, or Black rectangle backdrop behavior. Settings take effect at runtime and are not localized or persisted. | Real tray-menu and background acceptance tests; manual tray check. |
| First Alt+Tab | A successful Alt+Tab opens the overlay and selects the first eligible candidate. An empty or failed opening leaves the native gesture usable. | Real session and executable acceptance tests; physical-hook check. |
| Sticky Alt release | In the default Sticky mode, releasing Alt deliberately leaves the overlay open for an explicit choice. Immediate release and held-then-released Alt have the same behavior. | Real session acceptance tests; physical-hook check. |
| Tap (classic) Alt release | In selectable Tap mode, releasing Alt activates the current selection and closes the overlay. If pointer movement cleared the selection, release leaves the session available rather than activating an arbitrary window. | Real session acceptance tests; physical-hook check. |
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
| Full desktop backdrop | In the default Full desktop mode, render Explorer's desktop host (`Progman`, or the matching `WorkerW`) into an off-screen bitmap containing wallpaper and icons. Do not capture the screen or compose the background from application windows. | Real shell-backdrop acceptance tests; manual Explorer/restart/RDP check. |
| Alternative backdrops | Background image only paints the desktop wallpaper/pattern without icons; Black rectangle paints a black owner background. Both are selectable from the tray and do not require shell capture. | Real tray/background acceptance tests; manual visual check. |
| First frame | Prepare the selected backdrop and DWM preview surfaces while the owner is hidden, then paint the owner synchronously as it is shown. The first composed preview frame must contain the live source rather than a delayed placeholder, and the backdrop must not show application pixels. Full desktop reuses the shell snapshot; image-only and black modes do not require shell capture. | Real process first-frame, thumbnail-reveal, and background-mode acceptance tests; manual visual check. |
| Shell fallback | If the shell host or render is unavailable in Full desktop mode, use the last matching frame when possible or black, release partial resources, and keep opening usable. Image-only and black modes remain independent of the retained shell snapshot. | Real shell-backdrop and background-mode acceptance tests; manual protected/RDP check. |
| DWM previews | Configure destination geometry and make each thumbnail ready while its owner remains hidden, surface native failures, and unregister deterministically. This must not defer per-thumbnail source preparation until after the owner is visible. | Real DWM and thumbnail-reveal acceptance tests; manual DWM-disabled/protected-surface check. |
| Resource lifetime | Windows, thumbnails, icons, fonts, bitmaps, hook handles, tray resources, and mutexes are released on normal close, failed construction, and process exit. | Real cleanup/resource acceptance tests; manual repeated-session handle check. |
| Activation handoff | Restore minimized targets and attempt foreground activation without `AttachThreadInput`. The intentional deactivation during handoff is not treated as an external interruption. | Real activation acceptance tests; manual focus/taskbar check. |
| Interruption | Lock/unlock, desktop deactivation, tray exit, display/DPI change, and DWM composition changes reset or safely close the current session and input state. | Manual Windows matrix; selected real-session cleanup coverage. |

## Explicit caveats

- The Full desktop backdrop is a static shell frame prepared while idle; applications can change behind the overlay without changing that session's wallpaper/icon image. Image-only and black modes are painted directly when the owner is shown.
- Sticky Alt release is the default behavior. The initial candidate is not activated merely because Alt was released; choose with Tab/Shift+Tab, D1..D9, NumPad1..NumPad9, or the pointer, or cancel with Escape/Alt+F4. The tray-only Tap (classic) setting restores classic release-to-activate behavior.
- Background selection is tray-only and runtime-only: Full desktop is the default, while Background image only and Black rectangle are opt-in alternatives. No UI text is localized because the application has no other user-facing text surface.
- Protected, secure, unavailable, or capture-disabled surfaces may render black.
- A display/DPI topology change may close the session rather than rebuild every native tile in place.
- `SetForegroundWindow` can be denied by Windows focus/UIPI policy. The overlay remains admitted and usable when that happens.
- Physical hook behavior, unusual shell hosts, Explorer restarts, protected surfaces, and mixed monitor/DPI configurations still require manual validation.
