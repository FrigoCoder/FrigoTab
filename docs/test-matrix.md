# FrigoTab acceptance test matrix

The matrix separates deterministic acceptance tests from native-desktop checks that require a physical keyboard or a particular Windows shell state. It is the gate used during acceptance-test-driven development.

## Automated suite

The suite is made up of plain Rust integration tests. All 64 default tests execute against real application objects, native Windows resources, or the launched executable. Session behavior is always observed through an external `FrigoTab.exe` process, its real `WH_KEYBOARD_LL` hook, real session/preview HWNDs, exact fixture message logs, and screen pixels. The low-level non-keyboard families use real Win32, DWM, GDI, shell, and tray objects. There is no BDD/Gherkin layer, feature file, `LiveSession`, controller-fed acceptance harness, `WM_BEGIN_SESSION` path, fake session, or intentionally-red lane. Marked input exercises the production hook ledger but does not impersonate physical delivery to the previous foreground HWND; that boundary remains in the manual matrix. Every process scenario waits for the owner-window ready barrier before input and clears fixture logs after startup foreground changes. A 65th cursor-confinement scenario is compiled but explicitly ignored because it mutates a resource shared with the interactive desktop.

| Timestamped family | Count | Evidence boundary | Coverage |
| --- | ---: | --- | --- |
| `t20260914t211700z_001_real_switcher_acceptance_tests` | 15 | Launched executable, real hook, fixture HWNDs, and native resources | Opening, sticky Alt release, forward/reverse wrapping, number and pointer selection, cancellation, stale targets, activation recovery, key-up balancing, pass-through, cleanup, and reopening. |
| `t20260914t212400z_002_real_window_and_backdrop_acceptance_tests` | 7 | Real window, DWM, GDI, and shell objects | Candidate classification, Unicode titles, restored geometry, stale HWND layout, shell wallpaper/icon snapshot, staged DWM visibility, and repeated thumbnail disposal. |
| `t20260914t212400z_003_real_process_acceptance_tests` | 6 | Launched `FrigoTab.exe` | Hidden/resident startup, single-instance exit, real session HWND, full desktop session, pointer activation, and first-frame shell backdrop ordering. |
| `t20260914t223000z_004_real_visual_parity_acceptance_tests` | 3 | Launched `FrigoTab.exe` and real layered preview windows | Owned window topology, DWM tile coverage, selected-tile tint, measured title backing, icon rendering, and centered number rendering. |
| `t20260915t175100z_005_alt_tab_behavior_acceptance_tests` | 5 | Launched executable, real hook, tray popup, and fixture HWNDs | Default Sticky release, Tap (classic) initial/forward/reverse release activation, and release with no pointer selection. |
| `t20260915t175100z_006_tray_and_background_acceptance_tests` | 4 | Launched `FrigoTab.exe` and its real tray popup | Default menu checkmarks, tray-only Tap selection, Black rectangle painting, and Background image only painting. |
| `t20260915t212300z_007_thumbnail_reveal_performance_acceptance_tests` | 1 | Launched `FrigoTab.exe` and the real DWM compositor | A live preview source is present in the first composed owner frame without a delayed placeholder. |
| `t20260925t203500z_008_close_button_acceptance_tests` | 4 | Launched executable, real tray, session HWNDs, layered previews, and fixture sources | The tray submenu selects Always visible, On hover (Alt-Tab / Win-Tab), or Hidden; Always visible has the default black/white rendering, On hover shows a pointer-hovered white X without a background independently of keyboard selection, close requests target the exact source asynchronously without activation, and the live session refreshes, reflows, and renumbers after the source disappears while remaining open. |
| `t20260930t183700z_009_rapid_alt_tab_acceptance_tests` | 2 | Launched executable, marked real hook, fixture HWNDs, and foreground checks | An immediate normalized Tab-up stays consumed while Sticky remains visible and focused; after a rapid complete gesture is cancelled, a second Alt+Tab is admitted without leaking matching releases to the fixture windows. |
| `t20260930t195100z_010_alt_chord_isolation_acceptance_tests` | 2 | Launched executable, marked real hook, tray, and fixture HWNDs | Complete normalized Alt-down, Tab-down, Tab-up, and Alt-up sequences stay balanced in Sticky and Tap; Tap activates the real target while Sticky remains open. |
| `t20260930t204900z_011_held_alt_reopen_acceptance_tests` | 1 | Launched executable, marked real hook, real mouse input, and fixture HWNDs | Activating a real target while Alt remains logically held closes Sticky mode, and a distinct Tab reopens it before Alt release. |
| `t20260930t205300z_012_marked_hook_held_alt_acceptance_tests` | 1 | Launched `FrigoTab.exe`, real fixture HWNDs, and the real hook in explicit marked-input acceptance mode | Among injected transitions, the real `WH_KEYBOARD_LL` hook admits only explicitly marked test input, reopens Sticky mode after real activation while marked Alt remains held, and delivers zero keyboard messages—including the internal foreground nudge—to either fixture application. |
| `t20260930t212634z_013_native_keyboard_behavior_acceptance_tests` | 8 | Launched `FrigoTab.exe`, explicit marked `SendInput`, real `WH_KEYBOARD_LL`, fixture HWNDs, and exact `KeyboardMessage` logs | Bare-Alt and Alt+ordinary-key replay; Alt+Shift replay; Alt-first and Shift-first reverse suppression; native Ctrl+Alt; visible-session key quarantine; later native delivery; and wrong-marker rejection. Plain Right-Alt+Tab is intentionally manual because `SendInput` with `VK_RMENU` synthesizes Ctrl on layouts such as Hungarian. |
| `t20260930t232302z_014_window_group_acceptance_tests` | 2 | Real owner-linked Win32 top-level windows and production `WindowFinder` | An active detached tool window leaves its application root selectable and remains excluded itself; an ordinary active owned dialog remains the application-group representative. |
| `t20260930t232503z_015_external_window_lifecycle_acceptance_tests` | 1 | Launched `FrigoTab.exe`, real source HWNDs, DWM previews, hook input, foreground state, and screen pixels | A source destroyed independently of FrigoTab is removed from the visible sticky session; the preview graph is rebuilt and reflowed while surviving applications and foreground ownership remain intact. |
| `t20261001t075200z_016_fullscreen_application_acceptance_tests` | 2 + 1 attended | Launched executable, real hook, real Win32 messages, source HWNDs, and layered fallback; the ignored attended scenario additionally uses the shared cursor clip | The default tests cover debounced in-place relayout and no-redirection fallback/native close. The explicitly ignored attended scenario verifies that an active cursor clip is released while the session remains visible. |
| **Automated total** | **64** |  | **All non-disruptive automated tests pass before a publish is accepted.** |

The file prefixes are UTC timestamps recording when a family was introduced. Keep the prefix when refactoring a family; test functions remain descriptive plain Rust names. The process tests accept `FRIGOTAB_EXE` when a different built executable is being compared with the current Rust build. `RunningFrigoTab` starts the production executable and waits for its owner HWND to answer the ready barrier; `FixtureWindow` records the exact message, virtual-key, repeat, scan-code, extended, Alt-context, previous-state, and transition fields needed by the keyboard assertions.

## Keyboard injection boundary and alternatives

Family 013 and the marked portions of families 009–012 use real `SendInput` events carrying the private acceptance marker. Windows marks these transitions with `LLKHF_INJECTED`; only an explicit `--accept-marked-test-input` launch admits the exact marker, while normal production ignores injected and lower-integrity injected input. This is test-only admission and provenance filtering, not a security boundary. The tests exercise the real hook and external process, but marked input has important limits:

- Scheduling, batching, and typematic/autorepeat timing differ from a keyboard. Tests send explicit down/up/repeat transitions and use bounded waits, so they cannot prove hardware timing or hook behavior under load.
- Virtual-key, scan-code, extended-flag, keyboard-layout, and AltGr/right-Alt semantics can differ. The suite uses the documented message values and a physical Right-Alt case remains manual.
- Injected transitions can interact with modifiers already physically held. The production modifier/consumed-key ledgers are bounded and panic-safe; tests foreground each fixture, wait for readiness, and clear startup logs, but this does not emulate every physical-state race.
- Same-integrity `SendInput` does not prove UIPI/elevation or `LLKHF_LOWER_IL_INJECTED` behavior. Secure desktop/lock, RDP, true exclusive fullscreen/display-mode transitions, raw-HID consumers, Explorer/DWM state, hook timeout/removal, and heavy-load cases stay in the manual/special matrix.

`PostMessage`/`SendMessage` and UI Automation are less realistic for keyboard acceptance because they bypass global routing and `WH_KEYBOARD_LL`; they remain suitable only for deterministic window/menu setup. Virtual HID/VHF or a signed driver is closer to hardware, but requires administrator access, WDK/kernel code, driver signing/install, and ongoing Windows-version policy work. Physical USB HID or keyboard-robot input is the highest-fidelity option, but needs hardware and a manual lab. These alternatives do not replace the physical/special checks below.

## Manual Windows release matrix

Run these checks on each supported Windows configuration. Record the OS build, x64 architecture, DPI scale, monitor arrangement, DWM status, target elevation, and artifact used. Normal production launches ignore injected and lower-integrity injected keyboard events; the explicit `--accept-marked-test-input` mode accepts only the acceptance marker and is not a substitute for physical previous-foreground HWND delivery checks. Keep the physical/special matrix even when all 64 automated tests are green.

The Debug executable retains the historical ten-second `StartQuitTimer` safety timer. Use a Release artifact for sustained testing:

- `target/release/FrigoTab.exe` is the optimized native executable produced by Cargo.
- `artifacts/publish/win-x64/FrigoTab.exe` is the release artifact produced by `build.ps1 -Task Publish`.

Both artifacts are native x64 executables and do not require a managed runtime.

| Scenario | Pass condition |
| --- | --- |
| Tray startup, second launch, and Exit | No taskbar button; one instance owns the tray/hook; a second launch exits; Exit releases resources and the process. |
| Physical Alt+Tab | The overlay opens once, repeats and reverses correctly, remains responsive, and does not leave the previous foreground application with a swallowed key state. For normal Alt-first forward (`Alt down, Tab`) and reverse (`Alt down, Shift down, Tab`) chords, including the fastest possible release, the previous application receives no Alt, Tab, or deferred Shift transition. Also verify Shift-first reverse input receives a balanced Shift-up before foreground changes. Sticky (the default) leaves the overlay open on Alt release; tray-selected Tap (classic) activates the current selection on release. After Sticky activation or cancellation while Alt remains held, a distinct Tab must reopen the overlay before Alt release. |
| Key-up balance | Consumed Alt, Tab, Shift, digit, Escape, F4, and otherwise-unhandled session keys do not deliver unmatched key-ups. Verify ordinary Alt-only and Alt+key replay also have balanced transitions and retain their normal target behavior. |
| Alt fallback and modifier isolation | Exercise ordinary Alt-only menu activation and Alt+an ordinary key, then exercise normal Alt-first forward and reverse Alt+Tab. Verify Ctrl+Alt stays native for both an AltGr layout character and Windows' Ctrl+Alt+Tab behavior, while plain Right-Alt+Tab invokes FrigoTab. The switcher chord contributes no gated transitions to the previous application, and each non-switcher gesture remains balanced. |
| UIPI/elevation and focus denial | Ordinary/elevated boundaries do not join input queues; successful switches do not flash taskbar buttons; denied foreground activation leaves the overlay usable. |
| Number selection | D1..D9 and NumPad1..NumPad9 choose the intended tile on supported keyboard layouts without intermittent refusal. |
| Escape and Alt+F4 | The overlay closes without activating a target; the tray process stays alive. |
| Tray behavior and backdrop settings | Right-click the tray icon and verify the checked Sticky/Tap, Full desktop/Image only/Black rectangle, and Always visible/On hover/Hidden close-button items. Change each setting, open a new session, and verify the selected release behavior, backdrop, and close-button style; settings are runtime-only and reset after restart. |
| Close-button pointer behavior | In Always visible mode, click the 32x32 black/white-× control and verify that only its source receives asynchronous native `WM_SYSCOMMAND/SC_CLOSE`, the source is not activated, the Sticky session remains open, and selection clears. In On hover mode, verify that a background-free white × appears only on the pointer-hovered thumbnail. Select Hidden and verify the same point activates the tile normally. |
| Pointer hover/click/outside | Selection follows the pointer, clears outside tiles, keyboard navigation can recover selection, and a click activates exactly the intended target. |
| Minimized/maximized and stale targets | Restored placement chooses the correct monitor; closed targets do not crash the session; activation failure remains recoverable. |
| Mixed monitor/DPI topology | Negative origins, portrait layouts, DPI changes, resolution/orientation changes, and monitor add/remove close or rebuild safely without stale/off-screen UI. |
| Shell desktop wallpaper/icons | With Full desktop selected, the first visible owner frame shows the shell wallpaper and icons before any application preview and never shows ordinary application pixels. Opening does not perform a slow shell render in the Alt+Tab path, and a transient fullscreen mode does not replace the retained normal-desktop snapshot geometry. |
| Background image only / Black rectangle | With Background image only selected, the owner shows the desktop wallpaper/pattern without the covering application fixture; with Black rectangle selected, uncovered owner pixels are black. |
| Shell unavailable, DWM disabled, RDP, protected surfaces | Partial native resources are released; a matching prior shell frame or black fallback is used; the session remains usable. A source without DWM redirection retains an icon/title fallback and remains closable through native `SC_CLOSE`. |
| Native-resource stability | Repeated sessions, failed construction, DWM teardown, and tray Exit leave bounded GDI/native handle counts. |
| Lock/unlock and secure desktop | The active session and keyboard state reset; the next Alt+Tab works normally. |
| Fullscreen/borderless applications | Display/DPI/composition bursts are debounced and produce an in-place relayout while the visible Sticky session and retained backdrop geometry remain stable; periodic cursor unclipping lets the pointer leave the former game bounds. True exclusive display-mode/game transitions, including actual mode resets, remain manual and must not flash, quit FrigoTab, or leave stale/black UI. |
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

`Verify` and `Test` run all 64 non-disruptive acceptance tests serially and compile but skip the attended cursor-confinement scenario. `Publish` repeats the Release build and green test gate before producing `artifacts/publish/win-x64/FrigoTab.exe`. `Clean` removes generated Cargo and artifact output. `build.cmd` forwards the same arguments for callers that prefer a CMD entry point. No CI/CD service is required by this local workflow.

The cursor-confinement scenario is deliberately absent from those commands because it briefly restricts the real interactive-desktop pointer. Run it only as an attended manual check, with no fullscreen application active. Normal completion and panic unwinding call `ClipCursor(NULL)`; forcibly terminating the test process can bypass all in-process cleanup and must be avoided:

```powershell
cargo test -p frigotab-acceptance --release --test t20261001t075200z_016_fullscreen_application_acceptance_tests the_visible_switcher_releases_a_reapplied_system_cursor_clip -- --exact --ignored --test-threads=1
```
