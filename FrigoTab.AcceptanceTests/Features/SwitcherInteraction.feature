@Acceptance
Feature: Switcher interaction contract
  The switcher must keep the shell fail-open while providing deterministic
  keyboard and pointer behavior once a session is visible.

  @CurrentFeature @Regression
  Scenario: First Alt+Tab opens and selects the first candidate
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    Then the session is visible with candidate 0 selected
    And the keyboard event is consumed
    And the port opened 1 time

  @CurrentFeature @Regression
  Scenario: An empty candidate list fails open and cleans up
    Given an empty switcher port
    When I send Alt+Tab
    Then the keyboard event is passed through
    And the session is idle
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: An exception while opening fails open and cleans up
    Given a switcher port that throws while opening
    When I send Alt+Tab
    Then the keyboard event is passed through
    And the session is idle
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: An exception while selecting the initial candidate fails open and cleans up
    Given a switcher port with 3 candidates
    And selection will fail
    When I send Alt+Tab
    Then the keyboard event is passed through
    And the session is idle
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: An unrelated key is passed through while idle
    Given a switcher port with 3 candidates
    When I send an unrelated key-down
    Then the keyboard event is passed through
    And the port opened 0 times

  @CurrentFeature @Regression
  Scenario: An injected Alt+Tab is passed through
    Given a switcher port with 3 candidates
    When I send an injected Alt+Tab
    Then the keyboard event is passed through
    And the port opened 0 times

  @CurrentFeature @Regression
  Scenario: A key-up event is passed through while idle
    Given a switcher port with 3 candidates
    When I send a Tab key-up
    Then the keyboard event is passed through
    And the port opened 0 times

  @CurrentFeature @Regression
  Scenario: Repeated Alt+Tab advances and wraps forward
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send Alt+Tab 3 more times
    Then the session is visible with candidate 0 selected
    And the selected candidates were 0, 1, 2, 0

  @CurrentFeature @Regression
  Scenario: Shift+Alt+Tab moves backwards and wraps
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send Shift+Alt+Tab
    Then the session is visible with candidate 2 selected

  @CurrentFeature @Regression
  Scenario: Releasing Alt activates the selected candidate and closes
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send the Alt key-up
    Then candidate 0 was activated
    And the session is idle
    And the port was closed 1 time
    And the keyboard event is passed through

  @CurrentFeature @Regression
  Scenario: Escape cancels without activation
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send Escape key-down
    Then the session is idle
    And activation was not attempted
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: Alt+F4 cancels without activation
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send Alt+F4 key-down
    Then the session is idle
    And activation was not attempted
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: D1 selects and activates the first candidate
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send D1 key-down
    Then candidate 0 was activated
    And the session is idle

  @CurrentFeature @Regression
  Scenario: NumPad2 selects and activates the second candidate
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send NumPad2 key-down
    Then candidate 1 was activated
    And the session is idle

  @CurrentFeature @Regression
  Scenario: An invalid digit preserves the current selection
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send D9 key-down
    Then the session is visible with candidate 0 selected
    And activation was not attempted

  @CurrentFeature @Regression
  Scenario: Pointer hover selects the candidate under the pointer
    Given a switcher port with 3 candidates
    And the pointer hit test returns candidate 2
    When I send Alt+Tab
    And I move the pointer
    Then the session is visible with candidate 2 selected

  @CurrentFeature @Regression
  Scenario: Pointer click activates the candidate under the pointer
    Given a switcher port with 3 candidates
    And the pointer hit test returns candidate 1
    When I send Alt+Tab
    And I click the pointer
    Then candidate 1 was activated
    And the session is idle

  @CurrentFeature @Regression
  Scenario: Pointer movement outside every tile clears selection without activation
    Given a switcher port with 3 candidates
    And the pointer hit test returns no candidate
    When I send Alt+Tab
    And I move the pointer
    Then the session is visible with no candidate selected
    And activation was not attempted

  @CurrentFeature @Regression
  Scenario: Alt+Tab restores keyboard selection after the pointer clears it
    Given a switcher port with 3 candidates
    And the pointer hit test returns no candidate
    When I send Alt+Tab
    And I move the pointer
    And I send Alt+Tab
    Then the session is visible with candidate 0 selected
    And the selected candidates were 0, 0
    And the keyboard event is consumed

  @CurrentFeature @Regression
  Scenario: Shift+Alt+Tab restores the last selection after the pointer clears it
    Given a switcher port with 3 candidates
    And the pointer hit test returns no candidate
    When I send Alt+Tab
    And I move the pointer
    And I send Shift+Alt+Tab
    Then the session is visible with candidate 2 selected

  @CurrentFeature @Regression
  Scenario: An invalid pointer candidate closes the inconsistent session safely
    Given a switcher port with 3 candidates
    And the pointer hit test returns invalid candidate 99
    When I send Alt+Tab
    And I click the pointer
    Then the session is idle
    And activation was not attempted
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: A hit-test failure closes the session safely
    Given a switcher port with 3 candidates
    And hit testing will fail
    When I send Alt+Tab
    And I move the pointer
    Then the session is idle
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: A selection-clear failure closes the session safely
    Given a switcher port with 3 candidates
    And the pointer hit test returns no candidate
    And clearing selection will fail
    When I send Alt+Tab
    And I move the pointer
    Then the session is idle
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: Activation failure leaves the session visible for retry or cancel
    Given a switcher port with 3 candidates
    And activation will fail
    When I send Alt+Tab
    And I send the Alt key-up
    Then the session is visible with candidate 0 selected
    And activation was attempted 1 time
    And the port was closed 0 times

  @CurrentFeature @Regression
  Scenario: An activation exception leaves the session visible for retry or cancel
    Given a switcher port with 3 candidates
    And activation will throw
    When I send Alt+Tab
    And I send the Alt key-up
    Then the session is visible with candidate 0 selected
    And activation was attempted 1 time
    And the port was closed 0 times

  @CurrentFeature @Regression
  Scenario: An interruption returns the application to idle
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And the active session is interrupted
    Then the session is idle
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: A cleanup exception cannot leave the application active
    Given a switcher port with 3 candidates
    And closing will fail
    When I send Alt+Tab
    And I send Escape key-down
    Then the session is idle
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: A display change relayouts a visible session
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And the display topology changes
    Then the session is visible with candidate 0 selected
    And the port was relaid out 1 time

  @CurrentFeature @Regression
  Scenario: A relayout failure closes the visible session safely
    Given a switcher port with 3 candidates
    And relayout will fail
    When I send Alt+Tab
    And the display topology changes
    Then the session is idle
    And the port was closed 1 time

  @CurrentFeature @Regression
  Scenario: A closed session can be opened again
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send the Alt key-up
    And I send Alt+Tab again
    Then the session is visible with candidate 0 selected
    And the port opened 2 times
