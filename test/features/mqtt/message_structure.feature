Feature: MQTT Message Structure
  Every MQTT telemetry message must be valid JSON with the
  required metadata fields present and within valid ranges.

  Background:
    Given I have captured MQTT messages from a smoke test

  Scenario: All messages are valid JSON
    Then every message should be valid JSON

  Scenario: Required metadata fields are present
    Then every message should have a "msg_id" field
    And every message should have a "timestamp" field
    And every message should have a "micros" field

  Scenario: Message IDs are positive
    Then every msg_id should be positive

  Scenario: Micros values are in range
    Then every micros value should be between 0 and 999999

  Scenario: Timestamps are plausible Unix timestamps
    Then every timestamp should be plausible
