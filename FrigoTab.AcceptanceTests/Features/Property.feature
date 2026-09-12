@Acceptance @CurrentFeature @Regression
Feature: Observable property values
  A property is used by the legacy UI to publish meaningful state changes.

  Scenario: Equal assignment is silent and a changed assignment notifies once
    Given a string property with value "before"
    When I assign the property the same value "before"
    And I assign the property "after"
    Then property change notifications count is 1
    And the property changed from "before" to "after"

