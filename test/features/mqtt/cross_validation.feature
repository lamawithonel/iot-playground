Feature: RTT/MQTT Cross-Stream Validation
  The number of MQTT messages received should correlate
  with the number of publish events logged via RTT, and
  all timestamps should be consistent with the device's
  SNTP-synchronized epoch.

  Background:
    Given I have captured RTT output from a smoke test
    And I have captured MQTT messages from a smoke test

  Scenario: Publish counts match within tolerance
    When the RTT log shows publish events
    Then the MQTT message count should be within 10% of the RTT publish count

  Scenario: All timestamps follow the SNTP epoch
    When the device has synchronized via SNTP
    Then every MQTT timestamp should be at or after the device epoch
    And the last MQTT timestamp should be within the test window
