@Acceptance
Feature: Monitor-local grid layout
  A desktop layout is deterministic, keeps each preview in its monitor's
  working area, and preserves the source window's aspect ratio.

  @CurrentFeature @Regression
  Scenario: Four windows use a near-square grid
    Given monitor "primary" has bounds 0,0,1920,1080 and working area 0,0,1920,1040
    And layout window "one" is on monitor "primary" at 0,0 with size 800,600
    And layout window "two" is on monitor "primary" at 0,0 with size 800,600
    And layout window "three" is on monitor "primary" at 0,0 with size 800,600
    And layout window "four" is on monitor "primary" at 0,0 with size 800,600
    When I arrange the grid layout
    Then 4 tiles are produced
    And monitor "primary" has 2 columns and 2 rows of tiles
    And every produced tile is inside monitor "primary" working area

  @CurrentFeature @Regression
  Scenario: A wide source window keeps its aspect ratio
    Given monitor "primary" has bounds 0,0,1000,800 and working area 0,0,1000,800
    And layout window "wide" is on monitor "primary" at 0,0 with size 1600,900
    When I arrange the grid layout
    Then tile "wide" preserves its source aspect ratio within 0.02
    And tile "wide" is inside monitor "primary" working area

  @CurrentFeature @Regression
  Scenario: Tiles retain a margin from the monitor edges
    Given monitor "primary" has bounds 0,0,1000,1000 and working area 0,0,1000,1000
    And layout window "square" is on monitor "primary" at 0,0 with size 1000,1000
    When I arrange the grid layout
    Then tile "square" has a positive margin inside monitor "primary"

  @CurrentFeature @Regression
  Scenario: Each monitor receives its own independent grid
    Given monitor "left" has bounds -1920,0,1920,1080 and working area -1920,0,1920,1040
    And monitor "primary" has bounds 0,0,1920,1080 and working area 0,0,1920,1040
    And layout window "leftApp" is on monitor "left" at -1800,100 with size 800,600
    And layout window "rightApp" is on monitor "primary" at 100,100 with size 800,600
    When I arrange the grid layout
    Then 2 tiles are produced
    And tile "leftApp" has a negative screen origin
    And tile "leftApp" is inside monitor "left" working area
    And tile "rightApp" is inside monitor "primary" working area

  @CurrentFeature @Regression
  Scenario: Invalid source dimensions are ignored
    Given monitor "primary" has bounds 0,0,1920,1080 and working area 0,0,1920,1040
    And layout window "zeroWidth" is on monitor "primary" at 0,0 with size 0,600
    And layout window "negativeHeight" is on monitor "primary" at 0,0 with size 800,-1
    When I arrange the grid layout
    Then 0 tiles are produced
    And no tile is produced for "zeroWidth"
    And no tile is produced for "negativeHeight"

