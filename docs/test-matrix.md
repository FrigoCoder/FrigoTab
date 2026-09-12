# FrigoTab acceptance test matrix

The matrix keeps deterministic acceptance tests separate from native-desktop validation while keeping every automated test green. It is intentionally small enough to use during each acceptance-test-driven development cycle.

## Automated suite policy

The automated suite is plain C# MSTest only. There is no Reqnroll/Gherkin layer, no `.feature` file, no intentionally-red lane, and no separate command for known failures. The current baseline is 95 passing tests.

Each `[TestClass]` and matching source file begins with `TYYYYMMDDTHHMMSSZ_NNN_DescriptiveFamilyName`. The timestamp is an immutable UTC family record, the suffix is stable, and the current set contains 12 families ending at `_077`. Test methods use descriptive plain C# names. A naming-convention acceptance test scans the class/file convention and uniqueness.

The tests are acceptance/contract tests even where a fake or source probe is used:

| Timestamped family | Count | Evidence boundary | Coverage |
| --- | ---: | --- | --- |
| Switcher interaction (`_001`) | 31 | In-memory session port | First/failed/repeated/reverse Alt+Tab, pass-through, number and pointer selection, cancellation, activation recovery, interruption, cleanup, relayout, reopening, and admission recovery. |
| Grid layout (`_031`) | 5 | Pure geometry contract | Near-square grids, aspect ratio, margins, negative monitor origins, and invalid dimensions. |
| Observable property (`_036`) | 1 | State contract | Equal assignment is silent; changed assignment notifies once. |
| Production composition (`_037`) | 1 | Reflection contract | `SessionForm` implements `ISwitcherSessionPort`, owns `SwitcherApplication`, and exposes the hook entry point. |
| Win32 integration (`_038`) | 9 | Source/metadata contract | No display reset or synthetic activation, tray-only startup, rooted Debug safety timer, hook-install diagnostics, live no-capture DWM backdrop, best-effort foreground denial, dedicated hook message loop, subscriber isolation, and visual-tile pointer routing. |
| Gesture balance (`_041`) | 4 | In-memory session port | Consumed Tab, digit, Escape, and F4 key-downs consume their matching key-up events. |
| Win32 remediation (`_045`) | 10 | Reflection/source contract | Visible thumbnails, stale-window handling, restored-monitor selection, hook deferral, event-driven modifiers, DWM fallback, Unicode, pointer-sized declarations, disposal, and process admission. |
| Keyboard infrastructure (`_055`) | 20 | Event/queue policy | Event-derived Alt/Shift state, injected input, balanced suppression, auto-repeat, deferred delivery, bounded-queue fail-open, token-bound admission recovery, a reserved session-ending slot, reset-generation invalidation, complete forward/reverse replay with direction preserved across Shift changes, native `INPUT` structure size, dual-Shift tracking, native-Alt recovery, pending-admission abort, failed-post recovery, and dedicated hook-thread ownership. |
| DWM thumbnails (`_062`) | 5 | Fake native adapter | Visible opaque destination/source updates, returned-handle cleanup after registration failure, surfaced update failure with disposal, and unregister-failure diagnostics. |
| Single instance (`_065`) | 1 | Real named-mutex contract | A competing thread cannot acquire the application mutex. |
| Test naming convention (`_067`) | 1 | Repository contract | Timestamped class/file family names are unique and methods remain descriptive plain C# names. |
| DWM desktop backdrop (`_077`) | 7 | Fake native adapters | All-client-area glass margins, failure/recovery, missing destination, one `GetShellWindow` fallback thumbnail, black fallback for a missing source, registration-failure handle release, and idempotent fallback disposal. |
| **Total** | **95** |  | **All automated tests are green.** |

The source and metadata probes protect production invariants; they do not prove that every native call succeeds on every Windows desktop. The manual matrix below remains part of release evidence.

## Manual Windows release matrix

Run these checks on each supported Windows configuration. Record the OS build, x64 architecture, DPI scale, monitor arrangement, DWM status, whether the target process is elevated, and the artifact used.

The Debug executable intentionally retains the legacy `StartQuitTimer` 10-second safety timer. It is a development safeguard, not release behavior. Use the Release artifact for sustained interactive testing:

- `artifacts/publish/lean-win-x64/FrigoTab.exe` is a framework-dependent single file and requires the .NET 10 Windows Desktop Runtime.
- `artifacts/publish/portable-win-x64/FrigoTab.exe` is a compressed self-contained single file and requires no preinstalled runtime, but extracts native/runtime components and uses its extraction cache.

| Scenario | Pass condition |
| --- | --- |
| Tray startup, second launch, and tray Exit | No taskbar button; one instance owns the tray/hook; a second launch exits cleanly; Exit releases resources and the process. |
| Dedicated hook thread | The hook installs and pumps on its own native message loop; UI rendering/construction does not time out the hook; shutdown uninstalls it cleanly. |
| Alt+Tab from ordinary, console, and elevated applications | The overlay opens once, repeats and reverses correctly, remains responsive, and does not leave the shell or foreground app with a swallowed key state. |
| Key-up balancing and failed-open replay | Consumed Tab/digit/Escape/F4 gestures do not deliver unmatched key-up events; failed forward/reverse admission replays the correct complete native gesture. |
| UIPI/elevation and foreground denial | Hook admission, `SendInput` replay, and `SetForegroundWindow` behavior are verified across ordinary/elevated boundaries; foreground denial leaves the overlay usable and does not replay native Alt+Tab merely because focus was denied. |
| Number selection | D1..D9 and NumPad1..NumPad9 select the intended tile on the supported keyboard layouts. |
| Escape and Alt+F4 | The overlay closes without activation; the tray process remains alive. |
| Pointer hover/click and outside movement | Selection follows the pointer, clears outside tiles, keyboard selection recovers, and click activates exactly the intended target. Verify separate layered/transparent tile forms. |
| Minimized/maximized and stale targets | Restored placement chooses the correct monitor; closed targets do not crash the session; activation failure remains recoverable. |
| Mixed monitor/DPI topology | Negative origins, portrait layouts, DPI changes, resolution/orientation changes, and monitor add/remove close or rebuild safely without stale/off-screen UI. |
| Live DWM glass backdrop | The actual desktop, applications, taskbars, and live motion remain visible through the no-redirection owner; application thumbnails stay opaque and pointer input continues to reach the session. No `CopyFromScreen`, bitmap scaling, or per-window background reconstruction occurs. |
| Glass unavailable, DWM disabled, RDP, protected, and missing shell source | Failed glass uses one `GetShellWindow` shell thumbnail; if that also fails, the backdrop is black where supported; application thumbnails use the icon/title fallback and the session remains usable. |
| Native resource stability | Repeated sessions, failed construction, DWM updates/unregister, and tray exit leave bounded GDI/native handle counts. |
| Lock/unlock and secure-desktop transition | The active session and keyboard state reset; the next Alt+Tab works normally. |
| Fullscreen or borderless exclusive applications | No display-mode reset, double draw, permanent active state, or unexpected live/fallback backdrop behavior. |
| Candidate classification | UWP/packaged apps, shell/start menu, toolbars, and multiple windows match the intended eligible-window policy. |

## Local commands

`build.ps1` is the canonical local build entry point. Run from the repository root:

```powershell
.\build.ps1 -Task Restore
.\build.ps1 -Task Build -Configuration Debug
.\build.ps1 -Task Test
.\build.ps1 -Task Verify
.\build.ps1 -Task Publish -Configuration Release
.\build.ps1 -Task PublishPortable -Configuration Release
.\build.ps1 -Task Clean
```

`Verify` and `Test` run the complete 95-test green suite. `Publish` and `PublishPortable` enforce a Release build and green test run before producing their respective single-file artifacts. `Clean` removes generated `bin`, `obj`, and `artifacts` outputs. `build.cmd` forwards the same arguments for callers that prefer a CMD entry point. No CI/CD service is required by this local workflow yet.
