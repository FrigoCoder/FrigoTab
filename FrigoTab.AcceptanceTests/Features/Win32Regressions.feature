@Acceptance @Regression @ContractProbe
Feature: Native adapter regressions
  These executable architecture checks prevent removed legacy workarounds
  from returning. Interactive behavior is still covered by the manual Windows
  release matrix.

  @KI-DISPLAY-001
  Scenario: Opening a session preserves display modes
    Given the current Win32 capabilities
    Then display modes are preserved while opening a session

  @FT-START-001
  Scenario: Startup is explicitly tray-only
    Given the current Win32 capabilities
    Then the application uses an explicit tray-only message loop

  @KI-HOOK-STARTUP-001
  Scenario: A hook installation failure is reported to the user
    Given the current Win32 capabilities
    Then a hook installation failure has a user-facing diagnostic
