# FrigoTab

FrigoTab is an experimental Windows Alt-Tab replacement. It presents eligible application windows as a numbered, thumbnail-based switcher and keeps a tray icon for its lifetime.

The stabilization work is isolated on the `codex/gpt-atdd-stabilization` branch; `master` is not used for this work. The historical WinForms/Win32 prototype now has an SDK-style solution, a testable switcher policy, and a local acceptance-test-driven development loop. Native Windows behavior is still a separate release gate.

## Development loop

The acceptance requirements and test policy are documented in:

- [Requirements and acceptance behaviors](docs/requirements.md)
- [Acceptance test matrix](docs/test-matrix.md)
- [Architecture and test boundaries](docs/architecture.md)

The suite is plain C# MSTest. Reqnroll, Gherkin, `.feature` files, and a separate intentionally-red test lane are no longer used. There are currently 95 green acceptance/contract tests.

Each `[TestClass]` and its matching source file begins with `TYYYYMMDDTHHMMSSZ_NNN_DescriptiveFamilyName`. The timestamp is an immutable UTC record of when that test family was introduced; the family suffix is stable, and the current set has 12 families ending at `_077`. Test methods use descriptive plain C# names. Do not rewrite an existing family timestamp when tests are refactored; the naming-convention test enforces this policy.

`build.ps1` is the local build entry point; no CI/CD service is assumed yet:

```powershell
.\build.ps1 -Task Restore
.\build.ps1 -Task Build -Configuration Debug
.\build.ps1 -Task Test
.\build.ps1 -Task Verify
.\build.ps1 -Task Publish -Configuration Release
.\build.ps1 -Task PublishPortable -Configuration Release
.\build.ps1 -Task Clean
```

`Verify` and `Test` run the complete green MSTest suite. `Publish` and `PublishPortable` repeat the Release build and green test gate before producing their artifacts. `Clean` removes generated `bin`, `obj`, and `artifacts` outputs. `build.cmd` forwards the same arguments for callers that prefer a CMD entry point.

The Debug executable retains the historical `StartQuitTimer` 10-second safety timer for development. It is not release behavior; use a Release publish for sustained interactive testing. The hook owns a dedicated native message-loop thread: suppression admission is synchronous and bounded in the hook callback, while expensive enumeration/rendering/construction is deferred to the UI thread. Foreground activation is best effort. The native Windows checks in the matrix still require real HWNDs, hooks, DWM, focus, monitor/DPI changes, lock/unlock, UIPI/elevation, and pointer input.

The backdrop is an opaque, one-shot snapshot of the composed virtual desktop. While the switcher is still hidden, FrigoTab captures the screen once with a native `BitBlt`, keeps that image for the session, and paints it at native size behind the application thumbnails. The capture is never reconstructed from individual application windows, so the image includes the desktop, taskbar, and whatever was visible at the beginning of the gesture. A failed or protected capture uses a solid black fallback and keeps the gesture fail-open. The dedicated hook thread remains bounded while this UI-thread capture and paint work runs; multi-monitor, RDP, protected-surface, and supported-version behavior remains in the manual release matrix.

## Runtime and distribution

The original project targeted .NET Framework 4.7.1. The current SDK-style projects target `net10.0-windows`. Developers need the .NET 10 SDK selected by `global.json`; end users need one of the following packages:

| Command | Artifact | Target-machine requirement |
| --- | --- | --- |
| `Publish` | Lean single-file framework-dependent `win-x64`, approximately 0.22 MiB | .NET 10 Windows Desktop Runtime for x64 |
| `PublishPortable` | Compressed self-contained single-file `win-x64`, approximately 46.85 MiB | No preinstalled .NET runtime; the executable extracts native/runtime components and uses its extraction cache |

The publish configuration uses invariant globalization and keeps only the `en` satellite-resource policy because the application has no localized resource set. It deliberately does not enable trimming or Native AOT; WinForms/Win32 reflection and native-resource behavior should remain predictable while the project is stabilized.

A plain native Win32/C++ rewrite is possible, but it would be a second implementation of the tray UI, global hook, DWM thumbnails, window enumeration, DPI/monitor logic, resource lifetime, and test seams. Retaining C# and offering the portable self-contained package avoids a runtime prerequisite without taking on that rewrite. Packaging and Authenticode signing remain before release.
