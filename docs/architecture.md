# FrigoTab architecture and test boundaries

FrigoTab is a WinForms shell around Win32 and DWM adapters. The current branch has a real policy boundary: `SwitcherApplication` owns deterministic switcher behavior, while `SessionForm` implements `ISwitcherSessionPort` for the native UI. The acceptance suite can therefore exercise the interaction contract without installing a global hook, creating HWNDs, or changing display modes.

## Current component map

| Boundary | Current responsibility | Source |
| --- | --- | --- |
| Composition root | Starts a form-less WinForms `ApplicationContext`, owns lifetime, and wires the hook, tray, and single-instance guard | `FrigoTab/Program.cs`, `FrigoTab/SingleInstanceGuard.cs` |
| Tray adapter | Creates the visible `NotifyIcon` and Exit command | `FrigoTab/SysTrayIcon.cs` |
| Keyboard adapter | Installs `WH_KEYBOARD_LL` on a dedicated native message-loop thread, performs bounded synchronous suppression admission, and posts managed work to the UI thread | `FrigoTab/KeyHook.cs`, `FrigoTab.Core/KeyboardModifierState.cs`, `KeyboardSuppressionState.cs`, `DeferredKeyboardDispatcher.cs` |
| Switcher policy | Opens, selects, cancels, commits, balances key suppression, and safely closes a session | `FrigoTab.Core/SwitcherApplication.cs` |
| Production session port | Enumerates candidates, creates the live compositor backdrop and native tiles, maps pointer/layout events, and attempts foreground activation | `FrigoTab/SessionForm.cs`, `FrigoTab.Core/ISwitcherSessionPort.cs` |
| Window catalog | Enumerates and classifies eligible top-level application HWNDs | `FrigoTab/WindowFinder.cs` |
| Window adapter | Reads title/style/placement, skips stale candidates, restores, and attempts foreground activation | `FrigoTab/WindowHandle.cs` |
| Geometry | Assigns monitor-local grid rectangles using restored window rectangles | `FrigoTab.Core/GridLayout.cs`, `FrigoTab/Layout.cs`, `FrigoTab/LayoutScreen.cs` |
| Preview/tiles | Registers DWM thumbnails, requests visible opaque destination/source updates, diagnoses native failures, and falls back to icon/title overlays when DWM is unavailable | `FrigoTab/Thumbnail.cs`, `FrigoTab/DwmThumbnailApi.cs`, `ApplicationWindow.cs`, `LayerUpdater.cs`, `WindowIcon.cs` |
| Desktop backdrop | Extends DWM glass across a no-redirection owner to reveal the live desktop; falls back to one compositor-managed shell thumbnail sourced through `GetShellWindow` | `FrigoTab/DwmGlassBackdrop.cs`, `DwmDesktopBackdrop.cs`, `FrigoTab/SessionForm.cs` |
| Observable state | Publishes legacy selected/visible changes | `FrigoTab/Property.cs`, `ApplicationWindows.cs` |
| Acceptance boundary | Runs the same policy and native contracts through plain C# MSTest classes | `FrigoTab.AcceptanceTests/*.cs` |

The application still creates one `ApplicationWindow` form per selectable candidate. The old unified-overlay experiment documented by GitHub issue #26 and commits `81d1cd2`, `4332e82`, and `5a05167` is not an implemented architectural guarantee.

## Live compositor backdrop model

The primary backdrop makes `SessionForm` a non-layered `WS_EX_NOREDIRECTIONBITMAP` owner and calls `DwmExtendFrameIntoClientArea` with all margins set to `-1`. Its client pixels are deliberately left unpainted, including `WM_ERASEBKGND`, so DWM exposes the actual live desktop underneath. Application thumbnails remain opaque DWM visuals in that same destination HWND. FrigoTab does not call `CopyFromScreen`, allocate a full-screen bitmap, scale a captured frame, or rebuild the background from window thumbnails.

The design was validated in a disposable on-machine spike: pixels in an animated underlying window changed between captures while an opaque DWM thumbnail remained visible in the glass owner. If glass extension fails, `DesktopWindowSource` obtains the shell desktop through `GetShellWindow` and `DwmDesktopBackdrop` registers one full-owner thumbnail as a cheap fallback. If both DWM paths are unavailable, the owner attempts a solid black fallback and keeps opening fail-open. Composition changes close the current session so the next gesture constructs the appropriate path again. Visual fidelity still requires the manual Windows matrix.

## Test organization and naming

There are 95 green plain C# MSTest methods in 12 timestamped test families. Each `[TestClass]` and matching file follows `TYYYYMMDDTHHMMSSZ_NNN_DescriptiveFamilyName`; the family suffix is stable and the current families end at `_077`. Methods inside a family use descriptive C# names. A naming-convention acceptance test enforces the class/file family convention and uniqueness; refactoring must not rewrite an existing family timestamp.

The automated tests cover three boundaries:

1. **Behavioral fake-port acceptance tests** exercise the observable `SwitcherApplication` policy without native desktop state: fail-open, repeated and reverse Alt+Tab, key-up balancing, cancellation, number and pointer selection, selection recovery, activation failure, interruption, cleanup, relayout, reopening, and input-admission recovery.
2. **Production linkage and native contract probes** use reflection, source inspection, and fakes to protect the composition root, live-glass and shell-fallback backdrop selection, startup/display behavior, dedicated hook thread, synchronous suppression admission, subscriber-failure containment, UI-thread deferral, failed-open replay direction, native `INPUT` layout, dual-modifier/native-Alt recovery, DWM destination/source visibility and unregister/update diagnostics, stale-window handling, restored-monitor selection, Unicode and pointer-sized declarations, resource cleanup, single-instance guard, best-effort foreground activation, and visual-tile pointer routing. They are executable specifications, not proof that every native call succeeds on every desktop.
3. **Manual Windows scenarios** exercise real hooks, HWND races, DWM, focus, DPI, monitor topology, protected surfaces, lock/unlock, UIPI/elevation, and pointer routing. They remain part of release evidence because no deterministic test process can reproduce all shell conditions.

There is no Reqnroll/Gherkin layer, no `.feature` file, and no separate intentionally-red test lane or task. Defects selected for this stabilization pass and their adjacent regressions are represented by green tests; native desktop validation remains the final confidence step.

## Current boundary and future extraction targets

`ISwitcherSessionPort`/`SwitcherApplication` is implemented today. The following narrower ports remain useful architectural targets if future changes need stronger isolation; their first adapters may continue to call the existing Win32 APIs:

| Port | Intended production adapter | Acceptance-test fake |
| --- | --- | --- |
| `IKeyboardSource` | Dedicated low-level keyboard hook thread | Deterministic key-down/up/repeat stream |
| `IWindowCatalog` | `EnumWindows` plus DWM/style queries | Immutable candidate snapshots, including stale/closed candidates |
| `IDisplayTopology` | `Screen.AllScreens`, DPI/display notifications | Fixed monitor arrangements and topology changes |
| `IWindowActivator` | Restore and `SetForegroundWindow` | Success/failure and target-disappeared outcomes |
| `ISessionView` | WinForms/layered/DWM renderer | Recorded commands or an in-memory view |
| `IClock` | Development safety timer and other time-based behavior | Controlled time and timeout scenarios |

Extracting these ports should be driven by a new acceptance requirement, not by adding abstraction without a behavior to protect.

## Lifetime, input, and threading rules

- The dedicated hook thread owns hook installation, the native message loop, and synchronous bounded suppression admission; it must not perform enumeration, DWM setup, rendering, or form construction. One reserved queue slot ensures session-ending input is not displaced by Tab auto-repeat.
- The UI thread owns session forms, selection state, deferred input dispatch, and deterministic disposal.
- A session publishes its complete resource graph only after construction succeeds; partial construction is disposed immediately.
- DWM thumbnail handles, HDCs, HBITMAPs, fonts, icons, hook handles, mutexes, and `NotifyIcon` ownership are explicit and idempotent.
- Reverse P/Invoke callbacks must not let managed exceptions cross the native boundary.
- Native coordinates, DPI units, and monitor topology are represented at one boundary and converted deliberately.
- `SetForegroundWindow` is best effort. A foreground denial is logged while the admitted overlay remains usable; it is not treated as a reason to replay native Alt+Tab.
- A session close is safe after cancellation, target disappearance, lock/unlock, display change, DWM failure, or tray exit.

## Native validation caveats

The green contract probes protect source-level invariants, but they do not replace manual testing. In particular, validate the dedicated hook thread under load, synchronous suppression and replay under UIPI/elevated boundaries, foreground activation denial, stale HWND races, live DWM glass, shell-fallback and application thumbnails, black fallback behavior, mixed-DPI and negative-origin monitors, resource counts over repeated sessions, protected surfaces, and second-launch behavior. Pointer routing through separate layered/transparent tile forms is covered by a source contract but still requires visual verification on supported Windows configurations.
