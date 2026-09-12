# FrigoTab requirements and acceptance behaviors

This document is the behavior inventory for acceptance-test-driven development. The work is isolated on `codex/gpt-atdd-stabilization`; it must not be treated as a change directly on `master`.

## Test-suite vocabulary

- **Normal** — deterministic acceptance/specification tests that must be green in the default verification run.
- **KnownIssue** — an intentionally red regression/contract probe for a defect that is still open. It is run separately so the default suite remains meaningful.
- **ManualWindows** — a real interactive Windows scenario requiring HWNDs, keyboard hooks, DWM, focus, monitor topology, DPI, or user input. It is part of release evidence, not a replacement for a deterministic test.

The current automated result is 40 green acceptance tests and 14 intentionally red `KnownIssue` tests. Four red tests are behavioral fake-port input-balance scenarios; the other ten are native contract probes that inspect production linkage/source/metadata and do not simulate a complete desktop.

## Current intended features

| ID | Suite | Acceptance behavior | Current coverage |
| --- | --- | --- | --- |
| FT-START-001 | Normal contract; ManualWindows runtime | Startup runs a form-less WinForms message loop, shows a tray icon, has no taskbar button, and keeps the switcher hidden until a gesture. | `Win32Regressions.feature`; verify on real Windows. |
| FT-EXIT-001 | ManualWindows | Tray Exit closes any active session, disposes the hook/tray resources, and exits. | Manual release matrix. |
| FT-HOOK-001 | ManualWindows plus adapter review | A non-injected global keyboard notification carries the virtual key/modifiers, and an unhandled event reaches the next hook. | `KeyHook.cs`; native hook test still required. |
| FT-HOOK-002 | ManualWindows | Alt+Tab opens the switcher and leaves the shell available when opening cannot succeed. | Policy fail-open scenarios are green; real hook timing is manual. |
| FT-ENUM-001 | ManualWindows only | Window enumeration excludes invisible/disabled/cloaked/no-activate candidates, keeps app windows selectable, and places tool windows behind them. | No automated enumeration test exists yet; extract an `IWindowCatalog` seam before adding one. |
| FT-LAYOUT-001 | Normal | Each candidate receives a numbered, aspect-preserving tile inside its monitor's working area; independent monitors retain their own origins and margins. | `GridLayout.feature` and production `Layout.cs` linkage. |
| FT-UI-001 | ManualWindows | An eligible tile shows its DWM preview, title, icon, number, and selection highlight. | Manual release matrix; thumbnail visibility remains open. |
| FT-UI-002 | ManualWindows | Tool-window thumbnails render behind selectable application tiles in reverse enumeration order. | Manual release matrix. |
| FT-SELECT-001 | Normal; ManualWindows | Pointer hover selects the tile under the pointer; moving outside clears selection without activation. | Fake-port behavior scenarios plus native pointer regression check. |
| FT-SELECT-002 | Normal; ManualWindows | Clicking a selected tile restores the target when needed, activates it, and closes the session after successful activation. | Fake-port policy scenarios plus native focus check. |
| FT-KEY-001 | Normal; ManualWindows | D1..D9 select/activate the corresponding candidate. | Fake-port behavior scenarios plus native keyboard check. |
| FT-CANCEL-001 | Normal; ManualWindows | Escape cancels without activation and releases session resources. | Fake-port behavior scenarios plus native check. |
| FT-CANCEL-002 | Normal; ManualWindows | Alt+F4 cancels the session without terminating the tray process. | Green policy scenario plus native check. |
| FT-ACTIVATE-001 | Normal; ManualWindows | Minimized candidates use their restored placement for layout and are restored before foreground activation. | Activation policy is tested; restored-rectangle monitor selection remains open. |
| FT-PROP-001 | Normal | Equal property assignment is silent; a changed assignment emits one synchronous old/new notification. | `Property.feature`. |

The green switcher feature also protects repeated Alt+Tab wrapping, Shift reverse selection, injected/key-up pass-through, empty/exceptional opens, pointer selection recovery after a clear, invalid hit-test handling, activation failure/exception recovery, interruption, cleanup failure, display-change relayout, and reopening after a close. These scenarios use `SwitcherApplication` with `FakeSwitcherSessionPort`; they do not claim that every native adapter is correct.

## Corrective work completed on this branch

The following issues were converted into green behavior or production contract checks and are no longer part of the red work queue:

| ID | Current result |
| --- | --- |
| KI-FAILOPEN-001 | Open/initial-selection failure passes the key through, closes the partial session, and leaves the policy idle. |
| KI-HOTKEY-001 | Key-down/up/repeat and Shift are represented; repeated Alt+Tab advances deterministically. |
| KI-KEY-001 | Alt+F4 cancels only the switcher. |
| KI-KEY-002 | NumPad1..NumPad9 map to the same candidates as D1..D9. |
| KI-DISPLAY-001 | Session startup no longer resets display modes or sends a fabricated `WM_ACTIVATEAPP`. |
| KI-LIFECYCLE-001 | Session open/close is transactional at the policy boundary and cleanup cannot leave the controller active. |
| KI-ACTIVATE-001 | Activation failure leaves the session available for retry/cancel instead of closing unconditionally. |
| KI-HOOK-STARTUP-001 | Hook-install failure has a user-facing diagnostic path. |
| Startup/selection recovery | Startup uses a form-less `ApplicationContext`, and a pointer clear does not permanently disable keyboard selection. |

These fixes still need native Windows validation where the requirement has a `ManualWindows` row.

## Open requirements and KnownIssue probes

The native rows below each have one intentionally red contract probe in `Win32KnownIssues.feature`. The separate behavioral `KI-KEY-BALANCE-001` examples are in `SwitcherKnownIssues.feature`. Fixes should first make each probe green, then move the requirement into the normal acceptance/release suite.

| ID | Priority | Current gap | Expected behavior |
| --- | --- | --- | --- |
| KI-THUMB-001 | P0 | DWM thumbnail updates do not request visibility. | A successful session shows live previews, or a controlled fallback is reported when DWM is unavailable. |
| KI-KEY-BALANCE-001 | P1 | Consumed Tab, digit, Escape, and F4 key-downs currently let their matching key-ups through, including after the session closes. | Consume both halves of every consumed gesture so another application cannot receive an unmatched key-up. |
| KI-STALE-001 | P1 | A window can disappear between enumeration, layout, and activation; stale native results are not consistently handled. | Stale candidates are skipped/removed, native failures are reported, and the overlay remains recoverable. |
| KI-LAYOUT-MONITOR-001 | P1 | Layout takes restored dimensions from `GetRect`, but chooses the monitor separately with `Screen.FromHandle`; that can disagree for a minimized window. | A minimized candidate is assigned to the monitor containing its restored placement. |
| KI-HOOK-LATENCY-001 | P1 | Expensive session construction is reachable from the low-level hook callback. | The callback returns quickly; session work is posted/deferred and failure still preserves shell pass-through. |
| KI-HOOK-MODIFIER-001 | P1 | `LowLevelKeyboardProc` calls `GetAsyncKeyState`, although Windows invokes the callback before asynchronous key state is updated. | Modifier state is derived from the hook event stream and reset explicitly across desktop/session interruptions. |
| KI-DWM-FAILURE-001 | P1 | DWM registration/update failures have no controlled renderer fallback. | DWM failure is detected, diagnosed, and rendered through a documented fallback or safe close. |
| KI-INTEROP-UNICODE | P1 | Text-related P/Invokes are not consistently explicit Unicode. | All text Win32 entry points use Unicode declarations and preserve non-ASCII titles/classes. |
| KI-INTEROP-POINTER | P1 | Remaining native parameters still use non-pointer-sized declarations. | `LPARAM`, `WPARAM`, callback data, and `ULONG_PTR` values use `IntPtr`/`UIntPtr` as appropriate on x64. |
| KI-RESOURCE-001 | P2 | Fonts, icons, DWM/GDI handles, and partial construction are not all deterministically disposed on the UI thread. | Repeated sessions have bounded native/GDI handles, including when a constructor fails halfway through. |
| KI-INSTANCE-001 | P2 | There is no process-wide single-instance guard. | A second launch exits cleanly or focuses the existing tray/session instance. |

The pointer-routing question is deliberately **not** a confirmed defect. Separate layered/transparent application forms have a history of fragility, so pointer hit-testing remains a `ManualWindows` regression risk until it is verified on supported Windows configurations.
