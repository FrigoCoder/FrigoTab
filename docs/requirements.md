# FrigoTab requirements and acceptance behaviors

This document is the behavior inventory for acceptance-test-driven development. The work is isolated on `codex/gpt-atdd-stabilization`; it must not be treated as a change directly on `master`.

## Test policy

The repository contains 76 green plain C# MSTest methods. There is no Reqnroll/Gherkin layer, no `.feature` file, and no intentionally-red test lane or task. Every test method starts with `TYYYYMMDDTHHMMSSZ_NNN`: an immutable UTC introduction timestamp followed by a stable sequence suffix; the current sequence reaches `_076`. A naming-convention acceptance test enforces this rule, and existing timestamps are not changed during refactoring.

The tests are split by evidence, not by pass/fail status:

- **Behavioral acceptance tests** use `SwitcherApplication` with an in-memory session port. They protect externally visible interaction without creating HWNDs or installing hooks.
- **Production contract tests** inspect the executable's composition, source-level Win32 declarations, and resource seams, and use fakes for DWM/error paths. They protect architecture and failure handling but cannot prove every native call on every desktop.
- **ManualWindows checks** use real hooks, HWNDs, DWM, focus, monitor/DPI changes, lock/unlock, protected surfaces, and pointer input. They are release evidence, not a substitute for deterministic tests.

## Current behaviors

| Area | Requirement | Automated evidence | Manual evidence |
| --- | --- | --- | --- |
| Startup | Start a form-less WinForms message loop, acquire the per-user single-instance mutex, show a tray icon, avoid a taskbar button, and keep the switcher hidden until a gesture. | Composition/source contract tests and mutex test. | Start a Release build on a real desktop and launch a second copy. |
| Tray exit | Exit closes any active session, disposes the hook/tray/mutex resources, and ends the process. | Policy cleanup coverage. | Tray Exit and process-lifetime check. |
| Keyboard observation | A non-injected low-level keyboard event is normalized with key transition and modifier state; injected events do not alter physical state. Left/right modifiers remain correct when one side is released, and a native Alt flag can recover a missed transition until Alt-up. Unhandled input reaches the next hook. | Keyboard infrastructure acceptance tests. | Real hook from ordinary and elevated applications. |
| Hook responsiveness and replay | The native callback performs bounded admission/suppression only; enumeration, DWM setup, rendering, and activation run later on the UI thread through a bounded queue. If opening is rejected after Alt is released, a complete injected Alt+Tab or Alt+Shift+Tab gesture is replayed using the native `INPUT` structure; the captured forward/reverse direction is preserved even if Shift changes before dispatch. | Deferred-dispatch, replay, structure-size, and full-queue tests. | Large candidate set, rapid repeats, failed-open recovery, direction changes, and shell responsiveness. |
| First gesture | A successful Alt+Tab opens the overlay and selects the first candidate. Empty or failed opening passes the gesture through and cleans up. | Switcher interaction acceptance tests. | Explorer, console, elevated, and ordinary applications. |
| Repeated gesture | Repeated Alt+Tab advances and wraps; Shift+Alt+Tab moves backward and wraps. | Switcher interaction acceptance tests. | Physical Alt/Shift transitions, including missed-release recovery. |
| Gesture balancing | Every consumed key-down has its matching physical key-up consumed, including Tab, digits, Escape, and F4 after the session closes. | Keyboard suppression and switcher acceptance tests. | Verify no foreground application receives an unmatched key-up. |
| Number selection | D1..D9 and NumPad1..NumPad9 select the same one-based candidates; invalid digits leave the current selection unchanged. | Switcher interaction acceptance tests. | Number input on supported keyboard layouts. |
| Cancellation | Escape and Alt+F4 close only the overlay, do not activate a target, and do not terminate the tray process. | Switcher interaction acceptance tests. | Physical Escape and Alt+F4 sequences. |
| Pointer selection and routing | Hover selects the tile under the pointer; moving outside clears selection; keyboard selection can take control again; visual tile forms route pointer input to the session surface. | Switcher interaction and production pointer-routing contract tests. | Layered/transparent tile hit-testing on supported Windows versions. |
| Pointer activation | Clicking a tile activates exactly the target under the pointer and closes after successful activation. | Switcher interaction acceptance tests. | Focus/foreground behavior, including elevated targets. |
| Window candidates | Enumerate eligible visible application windows while excluding cloaked, disabled, invisible, no-activate, and tool-window candidates. | Production source/contract coverage for the current classifier. | Packaged apps, shell/start menu, toolbars, and multiple windows. |
| Stale windows | A candidate that disappears between enumeration, layout, and activation is skipped or fails recoverably without aborting other candidates. | Production source/contract coverage and failure-path tests. | Close targets during enumeration and immediately before activation. |
| Monitor layout | Use the restored rectangle to choose a monitor for minimized candidates; preserve negative monitor origins, working-area margins, stable numbering, and source aspect ratios. | Grid and layout acceptance tests plus production contract coverage. | Mixed-DPI, portrait, negative-origin, and monitor-add/remove configurations. |
| Static backdrop | Capture the composed virtual desktop once while the overlay is hidden and paint that image behind the tiles. Do not reconstruct the background from tool-window thumbnails. | Production snapshot contract test. | Protected surfaces, secure desktop, unavailable capture, and desktop changes during a session. |
| DWM preview | Request visible opaque DWM thumbnails for both destination and source updates; unregister handles deterministically and diagnose unregister failures; surface HRESULT failures and use the icon/title overlay fallback when DWM is unavailable. | Thumbnail acceptance tests and native contract coverage. | DWM disabled, RDP, protected windows, and repeated sessions. |
| Selection rendering | Show title, icon, number, and selected highlight for each tile; dispose fonts, icons, layered DCs, bitmaps, thumbnails, and forms deterministically, including partial construction. | Resource and thumbnail contract tests. | Repeat sessions while watching native/GDI handle counts. |
| Activation | Restore minimized targets, attempt foreground activation, and keep the session recoverable when activation fails. | Switcher policy tests. | Focus-stealing restrictions, elevated targets, and stale HWNDs. |
| Interruption | Lock/unlock, desktop deactivation, tray exit, display change, and DPI change reset or safely close the current session and input state. | Interruption/relayout policy tests. | Secure desktop, resolution/orientation/DPI changes, and fullscreen applications. |
| Observable property | Equal assignment is silent; a changed assignment emits one synchronous old/new notification. | Property acceptance test. | Not required. |

## Defects addressed by the current green suite

The current branch protects the selected stabilization fixes and their adjacent regressions: fail-open admission, balanced key suppression, event-derived and dual-side modifiers, native-Alt recovery, deferred hook work, complete forward/reverse failed-open replay with direction preserved across Shift changes, pointer-sized `INPUT` layout, startup without a stack-trace visibility hack, no display-mode reset or fabricated activation message, transactional construction/cleanup, stale-window skipping, restored-rectangle monitor selection, visible DWM destination/source thumbnail properties and unregister diagnostics, controlled DWM fallback, Unicode and pointer-sized declarations, deterministic native/GDI disposal, single-instance admission, visual-tile pointer routing, and the one-snapshot desktop backdrop.

These are green executable specifications, not a promise that a protected or unusual Windows surface behaves identically to an ordinary desktop. The manual matrix is intentionally retained for those cases.

## Explicit design caveats

- The desktop backdrop is static for one session. A desktop change behind the overlay is not continuously reflected.
- Protected, secure, or unavailable capture surfaces may render as a black backdrop; opening remains fail-open.
- A live display/DPI topology change currently closes the session safely rather than rebuilding every native tile in place.
- DWM failure falls back to the icon/title overlay; that fallback is intentionally less informative than a live thumbnail.
- Pointer routing through separate layered/transparent tile forms has a history of fragility and must be verified on supported Windows configurations; it is not currently documented as a reproducibly confirmed click-through bug.
- Future work can extract narrower keyboard, window-catalog, display-topology, activator, view, and clock ports when a new acceptance behavior requires stronger isolation.
