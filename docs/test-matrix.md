# FrigoTab acceptance test matrix

The matrix keeps deterministic acceptance tests separate from native-desktop validation while keeping every automated test green. It is intentionally small enough to use during each acceptance-test-driven development cycle.

## Automated suite policy

The automated suite is plain C# MSTest only. There is no Reqnroll/Gherkin layer, no `.feature` file, no intentionally-red lane, and no separate command for known failures. The current baseline is 76 passing tests, with the current timestamp sequence reaching `_076`. A naming-convention acceptance test scans the suite and enforces the immutable UTC prefix rule.

Every method name begins with `TYYYYMMDDTHHMMSSZ_NNN`, an immutable UTC introduction timestamp plus a stable sequence suffix. The timestamp is part of the test's historical identity and must not be rewritten when implementation or wording changes.

The tests are acceptance/contract tests even where a fake or source probe is used:

| Area | Count | Evidence boundary | Coverage |
| --- | ---: | --- | --- |
| Switcher interaction | 30 | In-memory session port | First/failed/repeated/reverse Alt+Tab, pass-through, number and pointer selection, cancellation, activation recovery, interruption, cleanup, relayout, and reopening. |
| Grid layout | 5 | Pure geometry contract | Near-square grids, aspect ratio, margins, negative monitor origins, and invalid dimensions. |
| Observable property | 1 | State contract | Equal assignment is silent; changed assignment notifies once. |
| Keyboard infrastructure | 13 | Event/queue policy | Event-derived Alt/Shift state, injected input, balanced suppression, auto-repeat, deferred delivery, bounded-queue fail-open, complete forward/reverse Alt+Tab replay with direction preserved across Shift changes, native `INPUT` structure size, dual-Shift tracking, and native-Alt recovery. |
| Production composition | 1 | Reflection contract | `SessionForm` implements `ISwitcherSessionPort`, owns `SwitcherApplication`, and exposes the hook entry point. |
| Startup/display/backdrop | 4 | Source/metadata contract | No display reset or synthetic activation, explicit tray-only startup, user-visible hook-install failure, and one pre-overlay desktop snapshot. |
| DWM thumbnails | 5 | Fake native adapter | Visible opaque destination/source updates, returned-handle cleanup after registration failure, surfaced update failure with disposal, and unregister-failure diagnostics. |
| Single instance | 1 | Real named-mutex contract | A competing thread cannot acquire the application mutex. |
| Native remediation | 10 | Reflection/source contract | Visibility, stale-window handling, restored-monitor selection, hook deferral, event-driven modifiers, DWM fallback, Unicode, pointer-sized declarations, disposal, and process admission. |
| Gesture balance | 4 | In-memory session port | Consumed Tab, digit, Escape, and F4 key-downs consume their matching key-up events. |
| Pointer routing | 1 | Production source contract | Visual tile forms route pointer input to the session surface. |
| Test naming convention | 1 | Repository contract | Every test method uses the immutable `TYYYYMMDDTHHMMSSZ_NNN` UTC prefix and a unique sequence suffix. |
| **Total** | **76** |  | **All automated tests are green.** |

The source and metadata probes protect production invariants; they do not prove that every native call succeeds on every Windows desktop. The manual matrix below remains part of release evidence.

## Manual Windows release matrix

Run these checks on each supported Windows configuration. Record the OS build, x64 architecture, DPI scale, monitor arrangement, DWM status, whether the target process is elevated, and the artifact used.

The Debug executable intentionally retains the legacy `StartQuitTimer` 10-second safety timer. It is a development safeguard, not release behavior. Use the Release artifact for sustained interactive testing:

- `artifacts/publish/lean-win-x64/FrigoTab.exe` is a framework-dependent single file and requires the .NET 10 Windows Desktop Runtime.
- `artifacts/publish/portable-win-x64/FrigoTab.exe` is a compressed self-contained single file and requires no preinstalled runtime, but extracts native/runtime components and uses its extraction cache.

| Scenario | Pass condition |
| --- | --- |
| Tray startup, second launch, and tray Exit | No taskbar button; one instance owns the tray/hook; a second launch exits cleanly; Exit releases resources and the process. |
| Alt+Tab from ordinary, console, and elevated applications | The overlay opens once, repeats and reverses correctly, remains responsive, and does not leave the shell or foreground app with a swallowed key state. |
| Key-up balancing | Consumed Tab, digit, Escape, and F4 gestures do not deliver unmatched key-up events to another application. |
| Number selection | D1..D9 and NumPad1..NumPad9 select the intended tile on the supported keyboard layouts. |
| Escape and Alt+F4 | The overlay closes without activation; the tray process remains alive. |
| Pointer hover/click and outside movement | Selection follows the pointer, clears outside tiles, keyboard selection recovers, and click activates exactly the intended target. Verify separate layered/transparent tile forms. |
| Minimized/maximized and stale targets | Restored placement chooses the correct monitor; closed targets do not crash the session; activation failure remains recoverable. |
| Mixed monitor/DPI topology | Negative origins, portrait layouts, DPI changes, resolution/orientation changes, and monitor add/remove close or rebuild safely without stale/off-screen UI. |
| Static desktop backdrop | The pre-overlay snapshot matches the visible desktop; protected/secure/unavailable surfaces use the documented black fallback; no tool-window reconstruction is expected. |
| DWM enabled, disabled, and RDP/protected windows | Live thumbnails are visible when available; icon/title fallback is controlled and the session remains usable when DWM fails. |
| Native resource stability | Repeated sessions, failed construction, DWM updates, and tray exit leave bounded GDI/native handle counts. |
| Lock/unlock and secure-desktop transition | The active session and keyboard state reset; the next Alt+Tab works normally. |
| Fullscreen or borderless exclusive applications | No display-mode reset, double draw, permanent active state, or unexpected backdrop behavior. |
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

`Verify` and `Test` run the complete 76-test green suite. `Publish` and `PublishPortable` enforce a Release build and green test run before producing their respective single-file artifacts. `Clean` removes generated `bin`, `obj`, and `artifacts` outputs. `build.cmd` forwards the same arguments for callers that prefer a CMD entry point. No CI/CD service is required by this local workflow yet.
