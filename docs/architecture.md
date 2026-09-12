# FrigoTab architecture and test boundaries

FrigoTab is a WinForms shell around Win32 and DWM adapters. The current branch has a real policy boundary: `SwitcherApplication` owns deterministic switcher behavior, while `SessionForm` implements `ISwitcherSessionPort` for the native UI. The acceptance suite can therefore exercise the interaction contract without installing a global hook, creating HWNDs, or changing display modes.

## Current component map

| Boundary | Current responsibility | Source |
| --- | --- | --- |
| Composition root | Starts a form-less WinForms `ApplicationContext`, owns lifetime, and wires the hook, tray, and single-instance guard | `FrigoTab/Program.cs`, `FrigoTab/SingleInstanceGuard.cs` |
| Tray adapter | Creates the visible `NotifyIcon` and Exit command | `FrigoTab/SysTrayIcon.cs` |
| Keyboard adapter | Installs `WH_KEYBOARD_LL`, derives left/right modifier state from hook transitions, bounds suppression, replays a complete failed-open gesture with its captured forward/reverse direction, and posts work to the UI thread | `FrigoTab/KeyHook.cs`, `FrigoTab.Core/KeyboardModifierState.cs`, `KeyboardSuppressionState.cs`, `DeferredKeyboardDispatcher.cs` |
| Switcher policy | Opens, selects, cancels, commits, balances key suppression, and safely closes a session | `FrigoTab.Core/SwitcherApplication.cs` |
| Production session port | Enumerates candidates, captures the backdrop, builds native tiles, maps pointer/layout events, and activates the selected HWND | `FrigoTab/SessionForm.cs`, `FrigoTab.Core/ISwitcherSessionPort.cs` |
| Window catalog | Enumerates and classifies eligible top-level application HWNDs | `FrigoTab/WindowFinder.cs` |
| Window adapter | Reads title/style/placement, skips stale candidates, restores, and attempts foreground activation | `FrigoTab/WindowHandle.cs` |
| Geometry | Assigns monitor-local grid rectangles using restored window rectangles | `FrigoTab.Core/GridLayout.cs`, `FrigoTab/Layout.cs`, `FrigoTab/LayoutScreen.cs` |
| Preview/tiles | Registers DWM thumbnails, requests visible opaque destination/source updates, diagnoses unregister/update failures, and falls back to icon/title overlays when DWM is unavailable | `FrigoTab/Thumbnail.cs`, `FrigoTab/DwmThumbnailApi.cs`, `ApplicationWindow.cs`, `LayerUpdater.cs`, `WindowIcon.cs` |
| Desktop backdrop | Captures the composed virtual desktop once, while the overlay is still hidden, and paints that static image behind the tiles | `FrigoTab/DesktopSnapshot.cs`, `FrigoTab/SessionForm.cs` |
| Observable state | Publishes legacy selected/visible changes | `FrigoTab/Property.cs`, `ApplicationWindows.cs` |
| Acceptance boundary | Runs the same policy and native contracts through plain C# MSTest classes | `FrigoTab.AcceptanceTests/*.cs` |

The application still creates one `ApplicationWindow` form per selectable candidate. The old unified-overlay experiment documented by GitHub issue #26 and commits `81d1cd2`, `4332e82`, and `5a05167` is not an implemented architectural guarantee.

## Desktop snapshot model

The background is intentionally one virtual-desktop snapshot captured before the overlay becomes visible. It replaces the former idea of reconstructing the desktop from tool-window thumbnails; tool windows are not rebuilt as a separate background layer. This gives the user a faithful starting image while keeping the native resource graph bounded and the overlay composition deterministic.

The snapshot is static for the lifetime of a session. A protected surface, secure desktop, unavailable capture path, or restricted application can produce a black fallback. A desktop change behind the overlay is not continuously reflected. Display/DPI topology changes are handled conservatively by closing the session rather than presenting stale overlay geometry; a future live rebuild would be a separate design change.

## What the automated tests mean

There are 76 green plain C# MSTest methods. Each name begins with the immutable UTC convention `TYYYYMMDDTHHMMSSZ_NNN`; the timestamp records test introduction, the current sequence reaches `_076`, and the name must not be rewritten when a test is refactored. A naming-convention acceptance test scans the suite and enforces this rule.

The tests cover three boundaries:

1. **Behavioral fake-port acceptance tests** exercise the observable `SwitcherApplication` policy without native desktop state: fail-open, repeated and reverse Alt+Tab, key-up balancing, cancellation, number and pointer selection, selection recovery, activation failure, interruption, cleanup, relayout, and reopening.
2. **Production linkage and native contract probes** use reflection, source inspection, and fakes to protect the composition root, startup/display behavior, keyboard queue, complete failed-open replay with forward/reverse direction preserved across Shift changes, native `INPUT` layout, dual-modifier/native-Alt recovery, DWM destination/source visibility and unregister/update diagnostics, stale-window handling, restored-monitor selection, Unicode and pointer-sized declarations, resource cleanup, desktop snapshot policy, single-instance guard, and visual-tile pointer routing. They are executable specifications, not proof that every native call succeeds on every desktop.
3. **Manual Windows scenarios** exercise real hooks, HWND races, DWM, focus, DPI, monitor topology, protected surfaces, lock/unlock, and pointer routing. They remain part of release evidence because no deterministic test process can reproduce all shell conditions.

There is no Reqnroll/Gherkin layer, no `.feature` file, and no separate intentionally-red test lane or task. Defects selected for this stabilization pass and their adjacent regressions are represented by green tests; native desktop validation remains the final confidence step.

## Current boundary and future extraction targets

`ISwitcherSessionPort`/`SwitcherApplication` is implemented today. The following narrower ports remain useful architectural targets if future changes need stronger isolation; their first adapters may continue to call the existing Win32 APIs:

| Port | Intended production adapter | Acceptance-test fake |
| --- | --- | --- |
| `IKeyboardSource` | Low-level keyboard hook | Deterministic key-down/up/repeat stream |
| `IWindowCatalog` | `EnumWindows` plus DWM/style queries | Immutable desktop snapshots, including stale/closed candidates |
| `IDisplayTopology` | `Screen.AllScreens`, DPI/display notifications | Fixed monitor arrangements and topology changes |
| `IWindowActivator` | Restore and `SetForegroundWindow` | Success/failure and target-disappeared outcomes |
| `ISessionView` | WinForms/layered/DWM renderer | Recorded commands or an in-memory view |
| `IClock` | Development safety timer and other time-based behavior | Controlled time and timeout scenarios |

Extracting these ports should be driven by a new acceptance requirement, not by adding abstraction without a behavior to protect.

## Lifetime and threading rules

- The UI thread owns session forms, selection state, input dispatch, and deterministic disposal.
- A session publishes its complete resource graph only after construction succeeds; partial construction is disposed immediately.
- DWM thumbnail handles, HDCs, HBITMAPs, fonts, icons, hook handles, mutexes, and `NotifyIcon` ownership are explicit and idempotent.
- Reverse P/Invoke callbacks must not let managed exceptions cross the native boundary.
- Native coordinates, DPI units, and monitor topology are represented at one boundary and converted deliberately.
- The low-level hook callback performs bounded admission/suppression only; enumeration, DWM setup, rendering, and activation are posted to the UI thread.
- A session close is safe after cancellation, target disappearance, lock/unlock, display change, DWM failure, or tray exit.

## Native validation caveats

The green contract probes protect source-level invariants, but they do not replace manual testing. In particular, validate the static-backdrop behavior on protected surfaces, DWM fallback visuals, hook responsiveness with a large candidate set, focus/foreground rules for elevated applications, stale HWND races, mixed-DPI and negative-origin monitors, resource counts over repeated sessions, and second-launch behavior. Pointer routing through separate layered/transparent tile forms remains a regression risk to verify on supported Windows configurations, not a claim that click-through is currently reproducibly broken.
