Feature: Error Absence
  The RTT log must not contain error patterns that indicate
  hardware failures, protocol errors, or panics.

  Background:
    Given I have captured RTT output from a smoke test

  Scenario: No device panic
    Then the RTT log should not match "(?i)panic"

  Scenario: No HardFault
    Then the RTT log should not match "(?i)hardfault|hard fault"

  Scenario: No TLS failure
    Then the RTT log should not contain "TLS handshake failed"

  Scenario: No MQTT connection failure
    Then the RTT log should not contain "MQTT CONNECT failed"

  Scenario: No publish failure
    Then the RTT log should not match "Publish #.*failed|publish.*failed"

  Scenario: No sensor read failure
    Then the RTT log should not match "SEN66: read failed|sensor.*error"

  Scenario: No SNTP sync failure
    Then the RTT log should not contain "All SNTP sync attempts failed"

  Scenario: No PUBACK poll failure
    Then the RTT log should not contain "MQTT poll failed awaiting PUBACK"

  Scenario: No sensor channel closure
    Then the RTT log should not contain "Sensor channel closed"

  Scenario: No broker rejection
    Then the RTT log should not match "rejected by broker"
