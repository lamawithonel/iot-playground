Feature: MQTT Timestamp Accuracy
  The device synchronizes its clock via SNTP.  All MQTT
  timestamps must be consistent with the device's SNTP
  epoch and fall within a reasonable time window.

  Background:
    Given I have captured MQTT messages from a smoke test

  Scenario: Timestamps are after the SNTP epoch
    When the device has synchronized via SNTP
    Then every MQTT timestamp should be at or after the device epoch

  Scenario: Last timestamp is within the test window
    When the device has synchronized via SNTP
    Then the last MQTT timestamp should be within 300 seconds of the device epoch
