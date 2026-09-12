@Acceptance @KnownIssue
  Feature: Switcher input requirements still under repair
  Global input suppression must never send another application an unmatched
  half of a keyboard gesture.

  @KI-KEY-BALANCE-001
  Scenario: A consumed Alt+Tab key-down also consumes its Tab key-up
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send a Tab key-up
    Then the keyboard event is consumed

  @KI-KEY-BALANCE-001
  Scenario: A consumed digit key-down also consumes its key-up
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send D1 key-down
    And I send D1 key-up
    Then the keyboard event is consumed

  @KI-KEY-BALANCE-001
  Scenario: A consumed Escape key-down also consumes its key-up
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send Escape key-down
    And I send Escape key-up
    Then the keyboard event is consumed

  @KI-KEY-BALANCE-001
  Scenario: A consumed Alt+F4 key-down also consumes its F4 key-up
    Given a switcher port with 3 candidates
    When I send Alt+Tab
    And I send Alt+F4 key-down
    And I send Alt+F4 key-up
    Then the keyboard event is consumed
