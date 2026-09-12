# FrigoTab architecture and test boundaries

FrigoTab is a WinForms shell around Win32 and DWM adapters. The current branch has a real policy boundary: `SwitcherApplication` owns the deterministic switcher state machine, while `SessionForm` implements `ISwitcherSessionPort` for the native UI. Acceptance tests can therefore exercise the interaction contract without installing a global hook, creating HWNDs, or changing display modes.

## Current component map

| Boundary | Current responsibility | Source |
| --- | --- | --- |
| Composition root | Starts a form-less WinForms `ApplicationContext`, owns lifetime, and wires the hook and tray | `FrigoTab/Program.cs` |
| Tray adapter | Creates the visible `NotifyIcon` and Exit command | `FrigoTab/SysTrayIcon.cs` |
| Keyboard adapter | Installs `WH_KEYBOARD_LL`, maps key messages, and contains callback failures | `FrigoTab/KeyHook.cs` |
| Switcher policy | Opens, selects, cancels, commits, and safely closes a session | `FrigoTab.Core/SwitcherApplication.cs` |
| Production session port | Builds native windows, maps pointer/layout events, and activates the selected HWND | `FrigoTab/SessionForm.cs` and `FrigoTab.Core/ISwitcherSessionPort.cs` |
| Window catalog | Enumerates and classifies app/tool/hidden HWNDs | `FrigoTab/WindowFinder.cs` |
| Window adapter | Reads title/style/placement and attempts restoration/activation | `FrigoTab/WindowHandle.cs` |
| Geometry | Assigns monitor-local grid rectangles | `FrigoTab.Core/GridLayout.cs`, `FrigoTab/Layout.cs`, `FrigoTab/LayoutScreen.cs` |
| Preview/overlay | Registers DWM thumbnails, draws title/icon/number/highlight | `FrigoTab/Thumbnail.cs`, `ApplicationWindow.cs`, `LayerUpdater.cs`, `WindowIcon.cs` |
| Coordinate adapter | Converts screen and client points | `FrigoTab/Points.cs`, `FrigoTab/Rect.cs` |
| Observable state | Publishes legacy selected/visible changes | `FrigoTab/Property.cs`, `ApplicationWindows.cs` |
| Acceptance boundary | Runs the same switcher policy with an in-memory fake port and separate source/metadata probes | `FrigoTab.AcceptanceTests/` |

The application still creates one `ApplicationWindow` form per selectable candidate. GitHub issue #26 and commits `81d1cd2`, `4332e82`, and `5a05167` document the abandoned/reverted unified-overlay experiment; it is not an implemented architectural guarantee.

## What the automated tests mean

The 40 green acceptance tests deliberately cover three different boundaries:

1. **Behavioral fake-port tests** (`SwitcherInteraction.feature`, `GridLayout.feature`, and `Property.feature`) exercise observable interaction and geometry rules without native desktop state. They cover Alt+Tab repeat/Shift behavior, fail-open, cancellation, number and pointer selection, selection recovery, activation failure, relayout, cleanup, and grid behavior.
2. **Production linkage/source/metadata contract probes** (`ProductionIntegration.feature` and `Win32Regressions.feature`) verify that the executable is wired to the tested policy and that removed startup/display workarounds do not return. They do not prove that a real hook, DWM thumbnail, HWND, focus change, or monitor topology works on a desktop.
3. **ManualWindows scenarios** exercise those native boundaries on supported Windows configurations. They remain required for release because no deterministic fake can validate the shell's hook timing, DWM, focus, DPI, or pointer routing.

The 14 red `KnownIssue` tests are split similarly: `SwitcherKnownIssues.feature` contains four behavioral `KI-KEY-BALANCE-001` input-suppression examples, while `Win32KnownIssues.feature` contains the ten native source/metadata probes listed below.

`FT-ENUM-001` is currently a ManualWindows requirement only. There is no automated window-enumeration acceptance test yet; introducing an `IWindowCatalog` seam is future work.

## Remaining target ports

`ISwitcherSessionPort`/`SwitcherApplication` is implemented today. The following ports are still architectural targets; their first adapters may continue to call the existing Win32 APIs:

| Port | Intended production adapter | Acceptance-test fake |
| --- | --- | --- |
| `IKeyboardSource` | Low-level keyboard hook | Deterministic key-down/up/repeat stream |
| `IWindowCatalog` | `EnumWindows` plus DWM/style queries | Immutable snapshots, including stale/closed candidates |
| `IDisplayTopology` | `Screen.AllScreens`, DPI/display notifications | Fixed monitor arrangements and topology changes |
| `IWindowActivator` | Restore and `SetForegroundWindow` | Success/failure and target-disappeared outcomes |
| `ISessionView` | WinForms/layered/DWM renderer | Recorded commands or an in-memory view |
| `IClock` / `IProcessInstance` | Timeout and single-instance guard | Controlled time and second-launch scenarios |

Extracting these ports should be driven by a failing acceptance requirement, not by increasing abstraction for its own sake.

## Lifetime and threading rules

- The UI thread owns session forms, selection state, input dispatch, and deterministic disposal.
- A session publishes its complete resource graph only after construction succeeds; partial construction is disposed immediately.
- DWM thumbnail handles, HDCs, HBITMAPs, fonts, icons, hook handles, and `NotifyIcon` ownership must be explicit and idempotent.
- Reverse P/Invoke callbacks must not let managed exceptions cross the native boundary.
- Native coordinates, DPI units, and monitor topology must be represented at one boundary and converted deliberately.
- Low-level hook callbacks must return quickly. Deferring expensive session construction is a remaining requirement (`KI-HOOK-LATENCY-001`).
- A session close must be safe after cancellation, target disappearance, lock/unlock, display change, DWM failure, or tray exit.

## Native boundary work queue

The ten native intentionally red `KnownIssue` probes are source/metadata contract checks for these remaining issues:

- DWM thumbnail visibility is not requested (`KI-THUMB-001`).
- Stale HWNDs are not skipped robustly during layout/activation (`KI-STALE-001`).
- Minimized windows are not assigned by their restored rectangle (`KI-LAYOUT-MONITOR-001`).
- Expensive session work still occurs from the low-level hook path (`KI-HOOK-LATENCY-001`).
- Modifier mapping calls `GetAsyncKeyState` before Windows updates asynchronous state (`KI-HOOK-MODIFIER-001`).
- DWM registration/update failure has no controlled fallback (`KI-DWM-FAILURE-001`).
- Text P/Invokes are not consistently explicit Unicode (`KI-INTEROP-UNICODE`).
- Remaining native parameters are not all pointer-sized (`KI-INTEROP-POINTER`).
- Native/GDI resources and partial construction are not yet deterministically disposed (`KI-RESOURCE-001`).
- There is no process-wide single-instance guard (`KI-INSTANCE-001`).

The pointer-routing question is deliberately **not** listed as a confirmed defect. Separate layered/transparent application forms and SessionForm mouse handlers have been historically fragile (commits `27c084e`, `e950810`, `11d712c`, and `300eb95`), so it remains a `ManualWindows` regression risk until verified on supported Windows configurations.
