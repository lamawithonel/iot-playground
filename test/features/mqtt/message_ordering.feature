Feature: MQTT Message Ordering
  Messages must arrive in order with strictly increasing
  IDs and non-decreasing timestamps.

  Background:
    Given I have captured MQTT messages from a smoke test

  Scenario: First message ID is 1
    Then the first msg_id should be 1

  Scenario: Message IDs are strictly increasing
    Then msg_id values should be strictly increasing

  Scenario: Timestamps are non-decreasing
    Then timestamps should be non-decreasing
