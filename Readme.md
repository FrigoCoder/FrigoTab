# FrigoTab

FrigoTab is an experimental Windows Alt-Tab replacement. It shows eligible application windows as numbered previews, keeps a tray icon while it runs, and leaves the native desktop untouched behind its overlay.

The current implementation is a small WinForms/Win32 application. The behavior baseline is maintained with plain C# MSTest acceptance tests that create real forms and HWNDs, exercise the real DWM and shell APIs, and launch the real executable. There are no Cucumber/Reqnroll feature files in this gate.

## Development loop

The behavior inventory, test boundary, and native validation checklist are documented in:

- [Requirements and acceptance behaviors](docs/requirements.md)
- [Acceptance test matrix](docs/test-matrix.md)
- [Architecture and test boundaries](docs/architecture.md)

There are 28 automated acceptance tests in three timestamped test families:

- 15 real switcher/session tests using actual WinForms controls, HWNDs, window enumeration, layout, DWM, and activation.
- 7 real window, DWM, and shell-backdrop tests using native desktop objects.
- 6 black-box tests that start the published-style executable and inspect its real process and session window.

Each `[TestClass]` and matching source file begins with `TYYYYMMDDTHHMMSSZ_NNN_DescriptiveFamilyName`. The timestamp records when that family was introduced; the suffix is stable. Test methods use descriptive plain C# names. Keep the class/file timestamp when refactoring a family.

`build.ps1` is the local build entry point; no CI/CD service is assumed:

```powershell
.\build.ps1 -Task Restore
.\build.ps1 -Task Build -Configuration Debug
.\build.ps1 -Task Test
.\build.ps1 -Task Verify
.\build.ps1 -Task Publish -Configuration Release
.\build.ps1 -Task PublishPortable -Configuration Release
.\build.ps1 -Task Clean
```

`Verify` and `Test` build the solution and run all 28 acceptance tests. `Publish` and `PublishPortable` run the Release build and the same green acceptance gate before producing artifacts. `Clean` removes generated `bin`, `obj`, and `artifacts` directories. `build.cmd` forwards the same arguments for callers that prefer a CMD entry point.

The Debug executable retains the historical ten-second `StartQuitTimer` safety timer. It is for development only; use a Release publish for sustained interactive testing. Releasing Alt deliberately leaves the switcher open (sticky mode): the user can then use Tab/Shift+Tab, a number, or the mouse, and can cancel with Escape or Alt+F4. An immediate Alt release has the same sticky behavior. Foreground activation uses the historical input nudge and does not join another application's input queue with `AttachThreadInput`.

The backdrop is a retained snapshot of the Windows shell desktop, including wallpaper and desktop icons. FrigoTab asks Explorer's desktop host (`Progman`, or the matching `WorkerW`) to render into an off-screen native bitmap. It does not capture the screen or reconstruct a background by searching and composing application windows. The first owner frame is painted from that snapshot before DWM previews are made visible; if shell rendering is unavailable, the overlay uses a black fallback and remains usable.

## Runtime and distribution

The current SDK-style projects target `net10.0-windows` and build an x64 WinForms executable. Developers need the .NET 10 SDK selected by `global.json`.

| Command | Artifact | Target-machine requirement |
| --- | --- | --- |
| `Publish` | Lean single-file `win-x64` executable | .NET 10 Windows Desktop Runtime |
| `PublishPortable` | Compressed self-contained single-file `win-x64` executable | No preinstalled .NET runtime; native/runtime components are carried by the executable |

The project has no localized UI resources and enables invariant globalization for the publish. Trimming and Native AOT remain disabled while the WinForms/Win32 behavior is being stabilized. Authenticode signing and installer decisions are release work, separate from the local acceptance gate.

## Native validation

Injected keyboard input is intentionally ignored by the production low-level hook, so an automated test cannot impersonate every physical Alt/Tab transition. The automated suite covers the real HWND and executable behavior that can be made deterministic; the manual matrix records the remaining physical-hook checks: ordinary and elevated applications, modifier transitions, UIPI, focus denial, mixed-DPI monitors, Explorer restart, protected surfaces, lock/unlock, and repeated resource cleanup.
