Feature: MQTT Protocol
  The device must connect to the MQTT broker and begin
  publishing telemetry messages with QoS 1 delivery.

  Background:
    Given I have captured RTT output from a smoke test

  Scenario: MQTT broker connection
    Then the RTT log should match "MQTT connected|entering publish loop"

  Scenario: Channel-driven publish mode
    Then the RTT log should contain "channel-driven publish"

  Scenario: Telemetry publishing
    Then the RTT log should match "Publishing #|published"

  Scenario: QoS 1 PUBACK acknowledgment
    Then the RTT log should match "acknowledged.*PUBACK|PUBACK for"
