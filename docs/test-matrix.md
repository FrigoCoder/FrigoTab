# FrigoTab acceptance test matrix

The matrix separates deterministic acceptance tests from native-desktop checks that require a physical keyboard or a particular Windows shell state. It is the gate used during acceptance-test-driven development.

## Automated suite

The suite is made up of plain Rust integration tests. All 45 tests execute against real application objects, native Windows resources, or the launched executable. There is no BDD/Gherkin layer, feature file, fake session, or intentionally-red lane.

| Timestamped family | Count | Evidence boundary | Coverage |
| --- | ---: | --- | --- |
| `t20260914t211700z_001_real_switcher_acceptance_tests` | 15 | Real switcher/session HWNDs and native resources | Opening, sticky Alt release, forward/reverse wrapping, number and pointer selection, cancellation, stale targets, activation recovery, key-up balancing, pass-through, cleanup, and reopening. |
| `t20260914t212400z_002_real_window_and_backdrop_acceptance_tests` | 7 | Real window, DWM, GDI, and shell objects | Candidate classification, Unicode titles, restored geometry, stale HWND layout, shell wallpaper/icon snapshot, staged DWM visibility, and repeated thumbnail disposal. |
| `t20260914t212400z_003_real_process_acceptance_tests` | 6 | Launched `FrigoTab.exe` | Hidden/resident startup, single-instance exit, real session HWND, full desktop session, pointer activation, and first-frame shell backdrop ordering. |
| `t20260914t223000z_004_real_visual_parity_acceptance_tests` | 3 | Launched `FrigoTab.exe` and real layered preview windows | Owned window topology, DWM tile coverage, selected-tile tint, measured title backing, icon rendering, and centered number rendering. |
| `t20260915t175100z_005_alt_tab_behavior_acceptance_tests` | 5 | Real switcher/session HWNDs | Default Sticky release, Tap (classic) initial/forward/reverse release activation, and release with no pointer selection. |
| `t20260915t175100z_006_tray_and_background_acceptance_tests` | 4 | Launched `FrigoTab.exe` and its real tray popup | Default menu checkmarks, tray-only Tap selection, Black rectangle painting, and Background image only painting. |
| `t20260915t212300z_007_thumbnail_reveal_performance_acceptance_tests` | 1 | Launched `FrigoTab.exe` and the real DWM compositor | A live preview source is present in the first composed owner frame without a delayed placeholder. |
| `t20260925t203500z_008_close_button_acceptance_tests` | 4 | Real switcher/session HWNDs, layered previews, and the launched tray process | The tray submenu selects Always visible, On hover (Alt-Tab / Win-Tab), or Hidden; Always visible has the default black/white rendering, On hover shows a pointer-hovered white X without a background independently of keyboard selection, close requests target the exact source asynchronously without activation, and the live session refreshes, reflows, and renumbers after the source disappears while remaining open. |
| **Total** | **45** |  | **All automated tests pass before a publish is accepted.** |

The file prefixes are UTC timestamps recording when a family was introduced. Keep the prefix when refactoring a family; test functions remain descriptive plain Rust names. The process tests accept `FRIGOTAB_EXE` when a different built executable is being compared with the current Rust build.

## Manual Windows release matrix

Run these checks on each supported Windows configuration. Record the OS build, x64 architecture, DPI scale, monitor arrangement, DWM status, target elevation, and artifact used. The production hook ignores injected keyboard events, so these physical-hook cases are intentionally not replaced by synthetic input.

The Debug executable retains the historical ten-second `StartQuitTimer` safety timer. Use a Release artifact for sustained testing:

- `target/release/FrigoTab.exe` is the optimized native executable produced by Cargo.
- `artifacts/publish/win-x64/FrigoTab.exe` is the release artifact produced by `build.ps1 -Task Publish`.

Both artifacts are native x64 executables and do not require a managed runtime.

| Scenario | Pass condition |
| --- | --- |
| Tray startup, second launch, and Exit | No taskbar button; one instance owns the tray/hook; a second launch exits; Exit releases resources and the process. |
| Physical Alt+Tab | The overlay opens once, repeats and reverses correctly, remains responsive, and does not leave the foreground application with a swallowed key state. Sticky (the default) leaves the overlay open on Alt release; tray-selected Tap (classic) activates the current selection on release. |
| Key-up balance | Consumed Tab, digit, Escape, and F4 gestures do not deliver unmatched key-ups. |
| UIPI/elevation and focus denial | Ordinary/elevated boundaries do not join input queues; successful switches do not flash taskbar buttons; denied foreground activation leaves the overlay usable. |
| Number selection | D1..D9 and NumPad1..NumPad9 choose the intended tile on supported keyboard layouts without intermittent refusal. |
| Escape and Alt+F4 | The overlay closes without activating a target; the tray process stays alive. |
| Tray behavior and backdrop settings | Right-click the tray icon and verify the checked Sticky/Tap, Full desktop/Image only/Black rectangle, and Always visible/On hover/Hidden close-button items. Change each setting, open a new session, and verify the selected release behavior, backdrop, and close-button style; settings are runtime-only and reset after restart. |
| Close-button pointer behavior | In Always visible mode, click the 32x32 black/white-× control and verify that only its source receives asynchronous `WM_CLOSE`, the source is not activated, the Sticky session remains open, and selection clears. In On hover mode, verify that a background-free white × appears only on the pointer-hovered thumbnail. Select Hidden and verify the same point activates the tile normally. |
| Pointer hover/click/outside | Selection follows the pointer, clears outside tiles, keyboard navigation can recover selection, and a click activates exactly the intended target. |
| Minimized/maximized and stale targets | Restored placement chooses the correct monitor; closed targets do not crash the session; activation failure remains recoverable. |
| Mixed monitor/DPI topology | Negative origins, portrait layouts, DPI changes, resolution/orientation changes, and monitor add/remove close or rebuild safely without stale/off-screen UI. |
| Shell desktop wallpaper/icons | With Full desktop selected, the first visible owner frame shows the shell wallpaper and icons before any application preview and never shows ordinary application pixels. Opening does not perform a slow shell render in the Alt+Tab path. |
| Background image only / Black rectangle | With Background image only selected, the owner shows the desktop wallpaper/pattern without the covering application fixture; with Black rectangle selected, uncovered owner pixels are black. |
| Shell unavailable, DWM disabled, RDP, protected surfaces | Partial native resources are released; a matching prior shell frame or black fallback is used; the session remains usable. |
| Native-resource stability | Repeated sessions, failed construction, DWM teardown, and tray Exit leave bounded GDI/native handle counts. |
| Lock/unlock and secure desktop | The active session and keyboard state reset; the next Alt+Tab works normally. |
| Fullscreen/borderless applications | No display-mode reset, permanent active state, unexpected stale/black backdrop, or double draw. |
| Candidate classification | Packaged apps, shell/start menu, toolbars, and multiple windows match the intended eligible-window policy. |

## Local commands

`build.ps1` is the canonical local entry point:

```powershell
.\build.ps1 -Task Restore
.\build.ps1 -Task Build -Configuration Debug
.\build.ps1 -Task Test -Configuration Release
.\build.ps1 -Task Verify -Configuration Release
.\build.ps1 -Task Publish -Configuration Release
.\build.ps1 -Task Clean
```

`Verify` and `Test` run all 45 acceptance tests serially. `Publish` repeats the Release build and green test gate before producing `artifacts/publish/win-x64/FrigoTab.exe`. `Clean` removes generated Cargo and artifact output. `build.cmd` forwards the same arguments for callers that prefer a CMD entry point. No CI/CD service is required by this local workflow.
