@Acceptance @Regression @ContractProbe
Feature: Production composition linkage
  The executable must use the same switcher policy exercised by the behavioral
  acceptance suite, while native desktop behavior remains a manual test layer.

  Scenario: Session form is wired to the tested switcher policy
    Given the production session form type
    Then it implements the switcher session port
    And it owns a SwitcherApplication controller
    And it exposes the keyboard event entry point
