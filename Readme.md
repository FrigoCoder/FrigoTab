# FrigoTab

FrigoTab is a native Windows Alt-Tab replacement. It shows eligible application windows as numbered previews, keeps a tray icon while it runs, and leaves the native desktop untouched behind its overlay.

The application is implemented in Rust as one small Win32 executable. Its acceptance gate uses plain Rust integration tests that create real HWNDs, exercise the real DWM, GDI, and Explorer shell APIs, and launch the real executable. The tests are acceptance tests, not unit tests or a BDD framework.

## Development loop

The behavior inventory, test boundary, and native validation checklist are documented in:

- [Requirements and acceptance behaviors](docs/requirements.md)
- [Acceptance test matrix](docs/test-matrix.md)
- [Architecture and test boundaries](docs/architecture.md)

There are 31 automated acceptance tests in four timestamped test families:

- 15 real switcher/session tests using actual HWNDs, window enumeration, layout, DWM, activation, and cleanup.
- 7 real window, DWM, GDI, and shell-backdrop tests using native desktop objects.
- 6 black-box tests that start the real executable and inspect its process, session HWND, and first visible frame.
- 3 real visual-parity tests that inspect the owned layered preview windows and their rendered pixels.

The families are the four timestamped Rust integration-test files:

- `acceptance-tests/tests/t20260914t211700z_001_real_switcher_acceptance_tests.rs`
- `acceptance-tests/tests/t20260914t212400z_002_real_window_and_backdrop_acceptance_tests.rs`
- `acceptance-tests/tests/t20260914t212400z_003_real_process_acceptance_tests.rs`
- `acceptance-tests/tests/t20260914t223000z_004_real_visual_parity_acceptance_tests.rs`

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

`Verify` and `Test` build the selected Cargo profile and run all 31 acceptance tests serially. `Publish` runs the Release build and the same green acceptance gate, then places the executable at `artifacts/publish/win-x64/FrigoTab.exe`. `Clean` removes generated Cargo and artifact output. `build.cmd` forwards the same arguments for callers that prefer a CMD entry point.

The direct Cargo equivalents are:

```powershell
cargo fetch --locked
cargo build --workspace --release
$env:FRIGOTAB_EXE = (Resolve-Path target/release/FrigoTab.exe).Path
cargo test -p frigotab-acceptance --release -- --test-threads=1
```

The Debug executable retains the historical ten-second `StartQuitTimer` safety timer. It is for development only; use the Release executable for sustained interactive testing. Releasing Alt deliberately leaves the switcher open (sticky mode): the user can then use Tab/Shift+Tab, a number, or the mouse, and can cancel with Escape or Alt+F4. An immediate Alt release has the same sticky behavior. Foreground activation uses the historical input nudge and does not join another application's input queue with `AttachThreadInput`.

The backdrop is a retained snapshot of the Windows shell desktop, including wallpaper and desktop icons. FrigoTab asks Explorer's desktop host (`Progman`, or the matching `WorkerW`) to render into an off-screen native bitmap. It does not capture the screen or reconstruct a background by searching and composing it from application windows. The first owner frame is painted from that snapshot before DWM previews are made visible; if shell rendering is unavailable, the overlay uses a black fallback and remains usable.

## Runtime and distribution

The Release build is a native x64 Windows executable. Rust's MSVC static CRT setting embeds the C runtime in the executable, and the application uses Windows APIs directly through `windows-sys`. A target machine does not need .NET, the .NET Desktop Runtime, or any other managed runtime.

| Command | Artifact | Target-machine requirement |
| --- | --- | --- |
| `Build` / `Test` | `target/debug/FrigoTab.exe` or `target/release/FrigoTab.exe` | Windows x64; build-time Rust toolchain only |
| `Publish` | `artifacts/publish/win-x64/FrigoTab.exe` | Windows x64; no .NET or managed runtime |

The executable embeds the application icon and Windows manifest. There are no localized UI resources or runtime package dependencies. Authenticode signing and installer decisions are release work, separate from the local acceptance gate.

## Native validation

Injected keyboard input is intentionally ignored by the production low-level hook, so an automated test cannot impersonate every physical Alt/Tab transition. The automated suite covers the real HWND and executable behavior that can be made deterministic; the manual matrix records the remaining physical-hook checks: ordinary and elevated applications, modifier transitions, UIPI, focus denial, mixed-DPI monitors, Explorer restart, protected surfaces, lock/unlock, and repeated resource cleanup.
