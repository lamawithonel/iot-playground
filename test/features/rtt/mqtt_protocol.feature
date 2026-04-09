Feature: MQTT Protocol
  The device must connect to the MQTT broker and begin
  publishing telemetry messages.

  Background:
    Given I have captured RTT output from a smoke test

  Scenario: MQTT broker connection
    Then the RTT log should match "MQTT connected|entering publish loop"

  Scenario: Telemetry publishing
    Then the RTT log should match "Publishing #|published"
