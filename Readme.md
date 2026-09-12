# FrigoTab

FrigoTab is an experimental Windows Alt-Tab replacement. It presents eligible application windows as a numbered, thumbnail-based switcher and keeps a tray icon for its lifetime.

The stabilization work is isolated on the `codex/gpt-atdd-stabilization` branch; `master` is not used for this work. The historical WinForms/Win32 prototype now has an SDK-style solution, a testable switcher policy, and a local acceptance-test-driven development loop. Native Windows behavior is still a separate release gate.

## Development loop

The acceptance requirements and test policy are documented in:

- [Requirements and acceptance behaviors](docs/requirements.md)
- [Acceptance test matrix](docs/test-matrix.md)
- [Architecture and test boundaries](docs/architecture.md)

`build.ps1` is the local build entry point; no CI/CD service is assumed yet:

```powershell
.\build.ps1 -Task Restore
.\build.ps1 -Task Build -Configuration Debug
.\build.ps1 -Task Verify
.\build.ps1 -Task Test
.\build.ps1 -Task TestKnownIssues
.\build.ps1 -Task Publish -Configuration Release
.\build.ps1 -Task Clean
```

Development requires a .NET 10 SDK; `global.json` selects the supported SDK feature band. The published application is self-contained, so this developer prerequisite does not apply to end-user machines.

Use `-Task Restore`, `Build`, or `Clean` as needed; `build.cmd` forwards the same arguments. `Verify` runs the green acceptance gate (currently 40 tests). `TestKnownIssues` runs 14 deliberately failing tests: four behavioral input-balance scenarios and ten native contract probes for defects that are still being fixed. A red result there is expected and should be tracked, not hidden. `Publish` independently performs a Release build and green test run before producing the self-contained `win-x64` artifact. `Clean` removes generated `bin`, `obj`, and `artifacts` outputs. Windows-only hook, DWM, focus, DPI, monitor, lock/unlock, and pointer checks are listed as `ManualWindows` in the matrix.

The Debug executable retains the historical `StartQuitTimer` 10-second safety timer for development. It is not release behavior; use the Release self-contained publish for sustained interactive testing.

## Runtime and distribution

The original project targeted .NET Framework 4.7.1. The current SDK-style projects target `net10.0-windows` and can publish a self-contained `win-x64` artifact, so an end user does not need to install .NET separately. A plain native Win32/C++ rewrite is possible, but it would be a substantial second implementation: the tray UI, global hook, DWM thumbnails, window enumeration, DPI/monitor logic, resource lifetime, and test seams would all need to be reimplemented. Retaining C# and shipping self-contained is the lower-risk path for this codebase. Native Windows validation, packaging, and Authenticode signing remain before release.
