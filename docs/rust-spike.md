# Rust native spike

The Rust work lives on `codex/rust-spike`, based on the green
`codex/gpt-atdd-stabilization` C# baseline. It is additive: the C# executable
remains the behavior oracle and rollback path until the Rust implementation
passes both the automated requirements and the native Windows release matrix.

## Goals

- Produce a small native executable without a .NET runtime prerequisite.
- Keep switcher policy, input state, recovery, and layout portable and safe.
- Put every Win32 type and `unsafe` call behind one narrow adapter boundary.
- Preserve the existing hook, activation, and first-frame ordering rather than
  redesigning them during the language port.
- Discover runtime capabilities and handle failure; do not branch on Windows
  version strings or build numbers.

This is not an attempt to make a cross-platform Alt-Tab replacement. Global
Windows hooks, DWM thumbnails, Explorer desktop capture, foreground policy,
and notification-area integration are Windows facilities. Source portability
means that the policy crate and its acceptance tests build on other operating
systems and that no Windows handle or error type leaks into that policy.

## Workspace boundaries

| Crate | Boundary |
| --- | --- |
| `frigo-tab-core` | Safe, dependency-free Rust. Normalized input, modifier and suppression state, failed-admission recovery, switcher policy, and monitor-independent geometry. `unsafe` is forbidden. |
| `frigo-tab-win32` | Target-gated `windows-sys` adapter. Owns raw handles, callbacks, HRESULT/BOOL checks, message-loop primitives, DWM, GDI, tray, and mutex operations. It exposes a side-effect-free surface report for automated checks. |
| `frigo-tab-spike` | A small composition/probe executable. It must never install the production hook or compete with the C# application unless an explicit native smoke command requests a short-lived probe. |

The Windows crate uses only `windows-sys` and its `windows-link` helper. No GUI
framework, asynchronous runtime, logging framework, serializer, or test
framework is included.

The executable is not an Alt-Tab replacement yet. It does not wire the core
policy to a production hook, UI bridge, window catalog, renderer, or activation
adapter. Its hook smoke verifies registration and explicit removal only; it
does not claim that a keyboard callback was delivered.

## Compatibility policy

Current stable Rust toolchains officially support Windows 10 and later. That
is presently the distribution baseline even though many individual Win32 APIs
used here are older. The production port follows the capability rules below;
items not present in the probe remain acceptance requirements. The source does
not encode a particular Windows release:

- Entry-point or operation availability is treated as a capability.
- DWM composition and thumbnail failures fall back to an opaque title/icon
  presentation.
- Missing or failing Explorer capture retains a compatible cached frame or
  produces a black frame; it must not capture application windows.
- Newer per-monitor DPI entry points will be resolved dynamically and fall
  back to manifest/system-DPI behavior.
- `PrintWindow` with documented flags is the baseline. The value commonly
  called `PW_RENDERFULLCONTENT` is an opportunistic enhancement and cannot be
  the only capture path because Microsoft does not document it in the current
  `PrintWindow` contract.
- Numeric OS-version checks (`GetVersionEx`, registry build checks, and similar
  gates) are prohibited. Call availability and results decide behavior.

Feature-gating in `windows-sys` controls what is compiled; it does not make a
load-time import optional. APIs outside the supported toolchain baseline must
therefore be loaded explicitly with `LoadLibraryExW`/`GetProcAddress` before a
fallback can be meaningful.

References:

- [Microsoft windows-rs crate guidance](https://github.com/microsoft/windows-rs/blob/master/docs/crates/windows.md)
- [Rust Windows target support](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)
- [Run-time dynamic linking](https://learn.microsoft.com/en-us/windows/win32/dlls/using-run-time-dynamic-linking)
- [DWM thumbnail registration](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmregisterthumbnail)
- [Low-level keyboard hook behavior](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc)

## Local build loop

The portable policy gate uses the active Rust host and does not require a
Windows SDK or native adapter link:

```powershell
.\build.ps1 -Task RustCoreTest
.\build.ps1 -Task RustCoreVerify
```

The complete native workspace currently uses the installed MSVC Rust
toolchain explicitly. `rust-lld` is selected by the repository without an
absolute machine path:

```powershell
.\build.ps1 -Task RustRestore -RustToolchain stable-x86_64-pc-windows-msvc
.\build.ps1 -Task RustVerify -RustToolchain stable-x86_64-pc-windows-msvc
.\build.ps1 -Task RustNativeSmoke -RustToolchain stable-x86_64-pc-windows-msvc
.\build.ps1 -Task RustBuild -Configuration Release -RustToolchain stable-x86_64-pc-windows-msvc
```

`RustVerify` checks formatting, runs Clippy with warnings denied, checks and
builds the workspace, and runs its tests. `Cargo.lock` is committed. The
explicit `RustNativeSmoke` task briefly installs a pass-through hook and
registers an invisible shell thumbnail, then tears both down; it never consumes
input or shows a window. The installed GNU Rust host can run the core tests
with the bundled linker, but
`windows-sys` 0.61 uses raw-dylib import generation that needs additional GNU
binutils; it is not the native release path.

## Acceptance migration

The 98 C# tests remain authoritative during the spike. Their migration is not
a source-to-source rewrite:

- 59 policy/input/layout behaviors belong in portable Rust acceptance tests.
- 12 DWM-thumbnail and desktop-snapshot behaviors belong behind fake native
  adapter contracts.
- 26 C# reflection/source probes must become runtime contracts, ABI assertions,
  or build checks rather than Rust source-text tests.
- 1 named-mutex behavior remains a real, process-level Windows contract.
- The existing manual Windows matrix remains mandatory for physical hook,
  compositor, shell, DPI, UIPI/elevation, and foreground behavior.

Rust acceptance-suite timestamps live in test file/module names. Individual
test functions use ordinary descriptive names.

The initial core port covers the classified portable session, input, recovery,
layout, and recent-regression sequences. Native adapter and end-to-end parity
remain incomplete; a green initial spike is not production parity.

## Spike exit criteria

Before the Rust executable can replace the C# application it must prove, in
order:

1. A hidden owner/message-loop window, tray lifetime, and single-instance guard.
2. A dedicated `WH_KEYBOARD_LL` thread whose callback performs bounded state
   admission and transfers value events through a preallocated, non-blocking
   bridge to the UI thread; neither thread may share the current mutable
   admission model behind a lock.
3. Complete failed-admission replay and matching key-up suppression.
4. Window enumeration, stale-window skipping, restored-rectangle monitor
   selection, and negative virtual coordinates.
5. A cached Explorer wallpaper/icon frame painted before any DWM thumbnail is
   made visible.
6. Foreground handoff with the historical input nudge, no `AttachThreadInput`,
   no intermittent number-selection failure, and no taskbar flashing on a
   successful activation.
7. DWM/GDI/hook/icon/window resource stability over repeated sessions.
8. A native application manifest, GUI subsystem, and DPI initialization whose
   capability fallbacks preserve behavior across the supported Windows matrix.
9. A signed Release artifact measured for size, startup time, memory, and clean
   execution on the supported Windows test matrix.
