@Acceptance @KnownIssue
Feature: Native adapter release requirements
  These scenarios are intentionally red against the current Win32 adapter.
  Each one names a concrete release requirement so the red suite is an
  explicit work queue rather than an undefined or pending test.

  @KI-THUMB-001
  Scenario: DWM thumbnails request visibility
    Given the current Win32 capabilities
    Then DWM thumbnail visibility is requested

  @KI-STALE-001
  Scenario: Stale HWND candidates are skipped safely
    Given the current Win32 capabilities
    Then stale HWND candidates are skipped safely

  @KI-LAYOUT-MONITOR-001
  Scenario: A minimized window is assigned by its restored rectangle
    Given the current Win32 capabilities
    Then restored rectangles determine candidate monitors

  @KI-HOOK-LATENCY-001
  Scenario: Expensive session work is kept out of the low-level hook callback
    Given the current Win32 capabilities
    Then session opening work is deferred outside the low-level hook callback

  @KI-HOOK-MODIFIER-001
  Scenario: Modifier state does not depend on asynchronous state inside the hook
    Given the current Win32 capabilities
    Then hook modifier state is derived from the event stream

  @KI-DWM-FAILURE-001
  Scenario: DWM failure has a controlled fallback
    Given the current Win32 capabilities
    Then DWM failure has a controlled fallback

  @KI-INTEROP-UNICODE
  Scenario: Native string interop is explicitly Unicode
    Given the current Win32 capabilities
    Then native string interop is explicitly Unicode

  @KI-INTEROP-POINTER
  Scenario: Native pointer-sized values use pointer-sized declarations
    Given the current Win32 capabilities
    Then native pointer-sized values use pointer-sized declarations

  @KI-RESOURCE-001
  Scenario: Native and GDI resources have deterministic disposal
    Given the current Win32 capabilities
    Then native and GDI resources have deterministic disposal

  @KI-INSTANCE-001
  Scenario: The process has a single-instance guard
    Given the current Win32 capabilities
    Then the process has a single-instance guard
