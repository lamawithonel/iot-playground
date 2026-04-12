Feature: Sensor Conditioning
  The SEN66 sensor requires progressive conditioning periods
  before each measurement type becomes available.  The RTT
  log should report conditioning completion for each sensor
  within the expected timeframe.

  Background:
    Given I have captured RTT output from a smoke test

  Scenario: Conditioning process starts
    Then the RTT log should contain "conditioning"

  Scenario: Temperature and humidity conditioning
    Then the RTT log should match "Temp/RH conditioning complete" given at least 43 seconds

  Scenario: VOC conditioning
    Then the RTT log should match "VOC conditioning complete" given at least 95 seconds

  Scenario: Particulate matter conditioning
    Then the RTT log should match "PM conditioning complete" given at least 155 seconds

  @extended
  Scenario: CO2 conditioning
    Then the RTT log should match "CO.*conditioning complete"

  @full
  Scenario: NOx conditioning
    Then the RTT log should match "NOx conditioning complete"
