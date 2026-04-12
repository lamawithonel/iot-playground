Feature: MQTT Message Timing
  With channel-driven publishing the device publishes each
  sensor reading as it arrives.  Inter-message intervals
  should be consistent with the sensor sample rate.

  Background:
    Given I have captured MQTT messages from a smoke test

  Scenario: Median inter-message interval is near the sample rate
    When at least 5 messages have been captured
    Then the median inter-message interval should be within 50% of the sample interval

  Scenario: No excessive gaps after conditioning
    When at least 5 messages have been captured
    Then no inter-message gap should exceed 3 times the sample interval
