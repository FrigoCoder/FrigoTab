# FrigoTab acceptance test matrix

The matrix keeps deterministic behavior tests separate from native-desktop validation. It is intentionally small enough to use during each acceptance-test-driven development cycle.

## Suite policy

| Suite | Meaning | Default result |
| --- | --- | --- |
| **Normal** | Deterministic requirements and production linkage contracts that describe supported behavior. | Green; included by `Verify`/`Test`. |
| **KnownIssue** | Regression tests for defects still present. | Intentionally red until fixed; run separately with `TestKnownIssues`. |
| **ManualWindows** | Interactive checks for hooks, HWND lifetime, DWM, focus, monitor topology, DPI, lock/unlock, and real pointer input. | Not run by the default automated command; attach evidence to the acceptance change. |

The current automated baseline is 40 green acceptance tests and 14 intentionally red `KnownIssue` tests. `KnownIssue` is not a second definition of success: once an issue is fixed, move its scenario into the normal suite and remove the `KnownIssue` classification.

## Automated green matrix

| Feature | IDs / coverage | Boundary | Expected assertion |
| --- | --- | --- | --- |
| `SwitcherInteraction.feature` | Current switcher interaction scenarios | Behavioral fake port | Fail-open, first/repeated/Shift Alt+Tab, key-up/injected pass-through, D/NumPad selection, Alt release commit, Escape/Alt+F4 cancellation, pointer selection/clear/recovery, invalid hit tests, activation failure, interruption, cleanup, relayout, and reopening all follow the documented policy. |
| `GridLayout.feature` | FT-LAYOUT-001 | Behavioral layout contract | Tiles stay in monitor working areas, preserve aspect ratio/margins, retain negative monitor origins, and ignore invalid source rectangles. |
| `Property.feature` | FT-PROP-001 | Behavioral state contract | Equal assignment is silent; a changed assignment emits one old/new notification. |
| `ProductionIntegration.feature` | FT-START-001 wiring checks | Production linkage contract | `SessionForm` implements `ISwitcherSessionPort`, owns `SwitcherApplication`, and exposes the keyboard entry point. |
| `Win32Regressions.feature` | KI-DISPLAY-001, FT-START-001, KI-HOOK-STARTUP-001 | Production source/metadata contract | Display modes are not reset, startup uses an explicit tray-only message loop, and hook-install failure has a user-facing diagnostic path. These probes do not exercise a real desktop. |

`FT-ENUM-001` is intentionally absent from this table. Window classification is currently a ManualWindows requirement; no automated enumeration test exists until an `IWindowCatalog` seam is introduced.

## Intentionally red KnownIssue matrix

The red work queue contains four behavioral policy tests and ten native source/metadata contract probes:

| ID | Feature | Test kind | Expected assertion |
| --- | --- | --- | --- |
| KI-KEY-BALANCE-001 | `SwitcherKnownIssues.feature` | Four behavioral fake-port examples | Consumed Tab, digit, Escape, and F4 key-downs also consume their matching key-ups, so another application cannot receive an unmatched half of a gesture. |
| KI-THUMB-001 | `Win32KnownIssues.feature` | Source/metadata probe | DWM thumbnail updates request visibility. |
| KI-STALE-001 | `Win32KnownIssues.feature` | Source/metadata probe | Stale HWND candidates are skipped/recovered without aborting the session. |
| KI-LAYOUT-MONITOR-001 | `Win32KnownIssues.feature` | Source/metadata probe | Restored rectangles determine the monitor for minimized candidates. |
| KI-HOOK-LATENCY-001 | `Win32KnownIssues.feature` | Source/metadata probe | Expensive session work is deferred outside the low-level hook callback. |
| KI-HOOK-MODIFIER-001 | `Win32KnownIssues.feature` | Source probe | Modifier tracking does not call `GetAsyncKeyState` from a callback that precedes asynchronous state updates. |
| KI-DWM-FAILURE-001 | `Win32KnownIssues.feature` | Source/metadata probe | DWM failure has a diagnosed, controlled renderer fallback or safe close. |
| KI-INTEROP-UNICODE | `Win32KnownIssues.feature` | Reflection probe | Text-related native declarations are explicitly Unicode. |
| KI-INTEROP-POINTER | `Win32KnownIssues.feature` | Reflection probe | Pointer-sized native values use `IntPtr`/`UIntPtr` as appropriate on x64. |
| KI-RESOURCE-001 | `Win32KnownIssues.feature` | Source/metadata probe | Native/GDI resources and partial construction are disposed deterministically. |
| KI-INSTANCE-001 | `Win32KnownIssues.feature` | Source probe | A process-wide single-instance guard exists. |

The red suite is expected to fail while these requirements remain open. Its failures are deliberate work items, not flaky acceptance tests.

## Manual Windows release matrix

Run these on each supported Windows release configuration. Record OS build, architecture, DPI scale, monitor arrangement, DWM status, and whether the target process is elevated.

The Debug executable intentionally retains the legacy `StartQuitTimer` 10-second safety timer. That timer is a development safeguard, not release behavior; use the Release self-contained publish from `artifacts/publish/win-x64` for sustained interactive testing.

| ID | Scenario | Pass condition |
| --- | --- | --- |
| FT-START-001 / FT-EXIT-001 | Start once, inspect tray icon, exit from tray | No taskbar button; exit releases hook/tray resources and the process. |
| FT-HOOK-001 / FT-HOOK-002 | Alt+Tab from Explorer, console, elevated, and ordinary apps | Non-injected events are handled as designed; opening is responsive, occurs once, and the shell is not left in a swallowed-key state. |
| FT-UI-001 / FT-UI-002 / KI-THUMB-001 | App/tool windows with DWM thumbnails, and DWM unavailable/RDP | Preview, title, icon, number, highlight, and background order are visible; failure is diagnosed or follows the documented fallback. |
| FT-SELECT-001 / FT-SELECT-002 | Hover and click every tile, including edges | Selection follows the pointer; click activates exactly the intended target. |
| FT-KEY-001 / KI-KEY-BALANCE-001 | D1..D9, NumPad1..NumPad9, key-up sequences | Number keys select the intended target and do not leave other applications with unmatched key-up events. |
| FT-CANCEL-001 / FT-CANCEL-002 | Escape and Alt+F4 | Overlay closes without activation; tray process remains alive. |
| FT-ACTIVATE-001 / KI-LAYOUT-MONITOR-001 | Minimized and maximized targets | Restored placement chooses the correct monitor; target restores/activates without changing display mode. |
| KI-STALE-001 | Close a target during enumeration and just before activation | No crash; stale tile is removed or fails recoverably. |
| KI-HOOK-LATENCY-001 | Open with a large candidate set and observe hook responsiveness | Hook callback returns promptly; expensive work does not cause shell input loss. |
| KI-HOOK-MODIFIER-001 | Shift/Alt transitions, including lock/unlock and missed key-up recovery | Forward/reverse selection follows the physical modifiers without stale state. |
| KI-INTEROP-UNICODE / KI-INTEROP-POINTER | Non-ASCII titles/classes and x64 message/callback paths | Text is preserved and native values are not truncated. |
| KI-RESOURCE-001 | Repeat sessions, DWM updates, and failed construction | Native/GDI handle counts remain bounded; partial construction is released. |
| KI-INSTANCE-001 | Launch a second copy | It exits cleanly or focuses the existing tray/session instance. |
| Manual-MOUSE-001 | Pointer routing through separate layered/transparent tile forms | Hover/click behavior is verified on the target OS. This is a regression risk, not a confirmed click-through defect. |
| Manual-SYSTEM-001 | Lock/unlock and secure-desktop transition | Session state resets and the next Alt+Tab works. |
| Manual-FULLSCREEN-001 | Enter/leave fullscreen or borderless exclusive applications | No mode reset, double draw, or permanent active state. |
| FT-ENUM-001 / Manual-WINDOWFINDER-001 | UWP/packaged app, shell/start menu, GIMP toolbar, multiple Eclipse windows | Candidate set matches the documented Alt+Tab policy. |

## Local commands

`build.ps1` is the canonical local build entry point. Run from the repository root:

```powershell
.\build.ps1 -Task Restore
.\build.ps1 -Task Build -Configuration Debug
.\build.ps1 -Task Verify
.\build.ps1 -Task Test
.\build.ps1 -Task TestKnownIssues
.\build.ps1 -Task Publish -Configuration Release
.\build.ps1 -Task Clean
```

`Verify` is the green default (restore/build plus the 40 non-KnownIssue tests); `Test` runs the normal automated suite directly. `TestKnownIssues` is expected to return a failing result while these 14 examples remain red, so it should not be used as the release gate. `Publish` enforces a Release build and green test run, then produces the self-contained `win-x64` artifact. `Clean` removes generated `bin`, `obj`, and `artifacts` outputs. `build.cmd` forwards the same arguments for callers that prefer a CMD entry point.
