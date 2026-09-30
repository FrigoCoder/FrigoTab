# FrigoTab

FrigoTab is a native Windows Alt-Tab replacement. It shows eligible application windows as numbered previews, keeps a tray icon while it runs, and leaves the native desktop untouched behind its overlay.

The application is implemented in Rust as one small Win32 executable. Its acceptance gate uses plain Rust integration tests that create real HWNDs, exercise the real DWM, GDI, and Explorer shell APIs, and launch the real executable. The tests are acceptance tests, not unit tests or a BDD framework.

Production code is grouped under `src/composition`, `src/input`, `src/switcher`, `src/window`, `src/geometry`, `src/rendering`, `src/desktop`, `src/tray`, and `src/system`. Each named struct or enum has its own source file; package `mod.rs` files collect and re-export related types. The former flat library module paths remain as aliases in `src/lib.rs` for callers that already use them.

## Development loop

The behavior inventory, test boundary, and native validation checklist are documented in:

- [Requirements and acceptance behaviors](docs/requirements.md)
- [Acceptance test matrix](docs/test-matrix.md)
- [Architecture and test boundaries](docs/architecture.md)

There are 62 automated acceptance tests in fifteen timestamped test families:

- 15 tests launch the executable, drive real HWNDs through its real hook, and observe session windows, navigation, activation, cleanup, and key-up behavior.
- 7 tests exercise real Win32, DWM, GDI, shell, layout, and native-resource objects without a controller-fed test double.
- 6 tests launch the executable and inspect startup, single-instance behavior, session ownership, pointer activation, and the first desktop frame.
- 3 tests inspect the launched executable's real layered preview windows and rendered pixels.
- 5 tests verify Sticky and Tap (classic) Alt-release behavior through the executable and tray.
- 4 tests open the real tray popup and verify runtime backdrop selection and painting.
- 1 test verifies that a live DWM thumbnail is ready in the first composed owner frame.
- 4 tests verify close-button rendering, exact-source close, hover and hidden modes, live refresh, and the tray submenu.
- 2 tests verify immediate Tab-up handling and recovery for a second rapid Alt+Tab gesture through the launched executable and real hook.
- 2 tests verify complete Sticky and Tap Alt chords through the launched executable and real hook.
- 1 test verifies held-Alt reopening after activation through the launched executable.
- 1 test verifies the marked acceptance-input path and held-Alt reopening through the launched executable.
- 8 native keyboard-behavior tests verify ordered balanced replay, Alt-first and Shift-first reverse suppression, native Ctrl+Alt, quarantine, and marker filtering through the launched executable, real `WH_KEYBOARD_LL` hook, and exact fixture message logs. Plain Right-Alt+Tab remains manual because `SendInput` on layouts such as Hungarian synthesizes Ctrl with `VK_RMENU` and therefore cannot prove the physical AltGr distinction.
- 2 real-window tests verify that an active owned tool window cannot hide its application root, while an ordinary owned dialog remains the group representative.
- 1 launched-executable test verifies that an application which closes independently is removed from the live sticky session without disturbing surviving previews.

These are plain Rust acceptance tests. They do not use Cucumber/Gherkin, `LiveSession`, a controller-fed acceptance harness, or `WM_BEGIN_SESSION`; session behavior is observed only through the launched executable. The families are the fifteen timestamped Rust integration-test files:

- `acceptance-tests/tests/t20260914t211700z_001_real_switcher_acceptance_tests.rs`
- `acceptance-tests/tests/t20260914t212400z_002_real_window_and_backdrop_acceptance_tests.rs`
- `acceptance-tests/tests/t20260914t212400z_003_real_process_acceptance_tests.rs`
- `acceptance-tests/tests/t20260914t223000z_004_real_visual_parity_acceptance_tests.rs`
- `acceptance-tests/tests/t20260915t175100z_005_alt_tab_behavior_acceptance_tests.rs`
- `acceptance-tests/tests/t20260915t175100z_006_tray_and_background_acceptance_tests.rs`
- `acceptance-tests/tests/t20260915t212300z_007_thumbnail_reveal_performance_acceptance_tests.rs`
- `acceptance-tests/tests/t20260925t203500z_008_close_button_acceptance_tests.rs`
- `acceptance-tests/tests/t20260930t183700z_009_rapid_alt_tab_acceptance_tests.rs`
- `acceptance-tests/tests/t20260930t195100z_010_alt_chord_isolation_acceptance_tests.rs`
- `acceptance-tests/tests/t20260930t204900z_011_held_alt_reopen_acceptance_tests.rs`
- `acceptance-tests/tests/t20260930t205300z_012_marked_hook_held_alt_acceptance_tests.rs`
- `acceptance-tests/tests/t20260930t212634z_013_native_keyboard_behavior_acceptance_tests.rs`
- `acceptance-tests/tests/t20260930t232302z_014_window_group_acceptance_tests.rs`
- `acceptance-tests/tests/t20260930t232503z_015_external_window_lifecycle_acceptance_tests.rs`

The timestamp and family suffix are stable identifiers. Keep them when refactoring a family; test functions use descriptive plain Rust names.

`build.ps1` is the local build entry point; no CI/CD service is assumed:

```powershell
.\build.ps1 -Task Restore
.\build.ps1 -Task Build -Configuration Debug
.\build.ps1 -Task Test -Configuration Release
.\build.ps1 -Task Verify -Configuration Release
.\build.ps1 -Task Publish -Configuration Release
.\build.ps1 -Task Clean
```

`Verify` checks formatting, runs Clippy with warnings denied, builds the selected Cargo profile, and runs all 62 acceptance tests serially. `Test` builds and runs the acceptance tests. `Publish` runs the Release verification gate, then places the executable at `artifacts/publish/win-x64/FrigoTab.exe`. `Clean` removes generated Cargo and artifact output. `build.cmd` forwards the same arguments for callers that prefer a CMD entry point.

The direct Cargo equivalents are:

```powershell
cargo fetch --locked
cargo build --workspace --release
$env:FRIGOTAB_EXE = (Resolve-Path target/release/FrigoTab.exe).Path
cargo test -p frigotab-acceptance --release -- --test-threads=1
```

The Debug executable retains the historical ten-second `StartQuitTimer` safety timer. It is for development only; use the Release executable for sustained interactive testing. Sticky Alt release is the default: releasing Alt leaves the switcher open so the user can use Tab/Shift+Tab, a number, or the mouse, and can cancel with Escape or Alt+F4. The tray menu can select Tap (classic), which activates the current selection when Alt is released. An immediate Alt release follows the selected mode. These settings are runtime-only, are available from the tray menu only, and reset to their defaults on the next launch. Foreground activation uses the historical input nudge and does not join another application's input queue with `AttachThreadInput`.

Close buttons are selected from the tray icon's `Close buttons` submenu and are runtime-only. `Always visible` is the default: every thumbnail has a 32x32 black close button with a white `×` in its top-right corner. `On hover (Alt-Tab / Win-Tab)` uses the normal Windows-style treatment: a white `×` with no black background appears only on the thumbnail currently under the mouse pointer, independently of keyboard selection. `Hidden` removes the close affordance; the same region behaves as ordinary tile activation. Clicking a visible close affordance asynchronously requests `WM_CLOSE` for that thumbnail's exact source window without activating the source. FrigoTab watches all sources in a visible session, so a source closed either through this button or independently is removed before the live candidate list is reflowed and renumbered. The sticky session and foreground overlay remain active. The setting is neither persisted nor localized.

The default Full desktop backdrop is a retained snapshot of the Windows shell desktop, including wallpaper and desktop icons. FrigoTab asks Explorer's desktop host (`Progman`, or the matching `WorkerW`) to render into an off-screen native bitmap. The tray menu also offers Background image only (the desktop wallpaper/pattern without icons) and Black rectangle. None of these modes captures the screen or reconstructs a background by searching and composing it from application windows. DWM prepares the preview surfaces behind the hidden owner, then the selected backdrop is painted as the owner is shown; if shell rendering is unavailable, the overlay uses a black fallback and remains usable.

## Runtime and distribution

The Release build is a native x64 Windows executable. Rust's MSVC static CRT setting embeds the C runtime in the executable, and the application uses Windows APIs directly through `windows-sys`. A target machine does not need .NET, the .NET Desktop Runtime, or any other managed runtime.

| Command | Artifact | Target-machine requirement |
| --- | --- | --- |
| `Build` / `Test` | `target/debug/FrigoTab.exe` or `target/release/FrigoTab.exe` | Windows x64; build-time Rust toolchain only |
| `Publish` | `artifacts/publish/win-x64/FrigoTab.exe` | Windows x64; no .NET or managed runtime |

The executable embeds the application icon and Windows manifest. There are no localized UI resources or runtime package dependencies. Authenticode signing and installer decisions are release work, separate from the local acceptance gate.

## Native validation

Normal production launches intentionally ignore injected and lower-integrity injected keyboard input. The explicit `--accept-marked-test-input` acceptance launch admits only test `SendInput` events carrying FrigoTab's marker; FrigoTab's own unmarked replay remains ignored, so the test mode cannot recurse through recovery. The marker is an `LLKHF_INJECTED` provenance filter for acceptance coverage, not a security boundary. The suite therefore exercises the real `WH_KEYBOARD_LL` callback, real executable/session HWNDs, and exact fixture `WM_KEY*` logs, but it cannot claim to emulate a physical keyboard completely.

Marked `SendInput` differs from hardware in timing and typematic/autorepeat, scheduler batching, virtual-key/scan-code/extended-flag details, keyboard layout and AltGr behavior, and interaction with modifiers already held on the physical keyboard. The hook's bounded, panic-safe modifier and consumed-key ledgers, the executable readiness barrier, explicit fixture foregrounding, and per-scenario log clearing reduce those effects but do not remove them. UIPI/elevation boundaries, lower-integrity injection, secure desktop/lock screens, RDP, exclusive fullscreen, raw-HID consumers, Explorer/DWM state, hook timeout/removal, and heavy system load remain manual or special-environment checks.

`PostMessage`/`SendMessage` and UI Automation are useful for deterministic window or menu setup, but bypass global input routing and `WH_KEYBOARD_LL`, so they are less realistic keyboard alternatives. A virtual HID/VHF device or signed kernel driver is closer to hardware but requires administrator access, driver signing and installation, WDK/kernel code, and Windows-version policy maintenance. A physical USB HID device or keyboard robot is the highest-fidelity option, at the cost of hardware, lab setup, and manual/non-hermetic execution. Keep the physical/special-environment matrix as a release requirement alongside the 62 local acceptance tests.
